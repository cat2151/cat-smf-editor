//! Resolve the player only for a cache miss, respecting the explicit --player.
use cat_smf_editor::patch_reference::PatchReference;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

/// The player without a window, answering one request on its standard streams.
pub fn headless(explicit: Option<PathBuf>, flag: &str) -> Result<Command, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let player = super::selection::resolve(explicit, &executable, std::env::var_os("PATH"))?;
    let mut command = Command::new(player);
    cat_smf_editor::surge_data_home::restore_for_child(&mut command);
    command
        .arg(flag)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    Ok(command)
}

pub fn generate(reference: &PatchReference, explicit: Option<PathBuf>) -> Result<Vec<u8>, String> {
    let _span = cat_smf_editor::startup_trace::span("state_generate");
    let mut child = headless(explicit, "--restore-state")?
        .spawn()
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(reference).map_err(|e| e.to_string())?;
    let sent = child
        .stdin
        .take()
        .ok_or("Player stdin unavailable")?
        .write_all(&bytes);
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    sent.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Player restoration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stdout.is_empty() || output.stdout.len() > 16 * 1024 * 1024 {
        return Err("Player returned an invalid state size".into());
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires STATE_REQUEST_REFERENCE, STATE_REQUEST_WORK_DIR and RESIDENT_PLAYER"]
    fn headless_generation_and_disk_cache_restore_without_a_player() {
        use cat_smf_editor::{
            instrument_layers,
            model::{Music, Track},
            score::{layer_track::Patch, state_cache::Cache},
        };
        let reference: PatchReference = serde_json::from_slice(
            &std::fs::read(std::env::var_os("STATE_REQUEST_REFERENCE").unwrap()).unwrap(),
        )
        .unwrap();
        let player = Some(PathBuf::from(std::env::var_os("RESIDENT_PLAYER").unwrap()));
        let work = PathBuf::from(std::env::var_os("STATE_REQUEST_WORK_DIR").unwrap());
        let output = headless(player.clone(), "--patch-roots")
            .unwrap()
            .output()
            .unwrap();
        let mut roots: cat_smf_editor::patch_roots::Table =
            serde_json::from_slice(&output.stdout).unwrap();
        let start = std::time::Instant::now();
        let mut calls = 0;
        let mut first = Cache::new(Some(work.clone()));
        let state = first
            .resolve(&reference, |r| {
                calls += 1;
                generate(r, player)
            })
            .unwrap();
        assert_eq!(calls, 1);
        let cold = start.elapsed();
        first.save().unwrap();
        drop(first);
        let mut track = Track { events: vec![] };
        let settings = instrument_layers::Settings {
            layers: vec![instrument_layers::Layer {
                patch: Some(reference),
                ..Default::default()
            }],
            ..Default::default()
        };
        instrument_layers::write(&mut track, &settings, &mut roots).unwrap();
        let bytes = cat_smf_editor::smf_io::encode(&Music {
            format: 0,
            division: 480,
            tracks: vec![track],
        })
        .unwrap();
        let parsed = cat_smf_editor::ffi::parse(&bytes).unwrap();
        let start = std::time::Instant::now();
        let mut second = Cache::new(Some(work));
        let restored = Patch::resolve(
            instrument_layers::read(&parsed.tracks[0], &mut roots).unwrap(),
            |r| {
                second.resolve(r, |r| {
                    calls += 1;
                    generate(r, Some("X:/missing/player.exe".into()))
                })
            },
        )
        .unwrap();
        assert_eq!(calls, 1, "disk hit must not resolve/start player");
        assert_eq!(
            restored.layers[0].instrument.as_ref().unwrap().state,
            state.state
        );
        println!("Headless generation {cold:?}; new-session SMF + disk cache restoration {:?}; player calls cold=1 warm=0", start.elapsed());
    }
}
