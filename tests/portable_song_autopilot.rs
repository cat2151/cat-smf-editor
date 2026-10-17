//! Opens a real song whose patch references are root-relative (ADR 0008), picks
//! another patch, saves, and checks the saved references stay root-relative.
//!
//! Set `CAT_PORTABLE_SMF` to a song with instrument layers on track 2; without it
//! the test does nothing. The editor runs with its own LOCALAPPDATA and TEMP, and
//! works on a copy of the song.
#![cfg(windows)]

#[path = "resident_autopilot/support.rs"]
#[allow(dead_code)]
mod support;

use cat_smf_editor::patch_roots::Table;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use support::*;

#[test]
#[ignore = "starts the real editor and player: windows open and audio plays"]
fn a_root_relative_song_opens_and_saves_root_relative() {
    let Some(source) = std::env::var_os("CAT_PORTABLE_SMF") else {
        println!("CAT_PORTABLE_SMF is not set; skipped");
        return;
    };
    let source = Path::new(&source);
    let name = source.file_name().unwrap();
    let work = std::env::temp_dir().join(format!("cat-portable-{}", std::process::id()));
    let (local, temp, songs) = (work.join("local"), work.join("temp"), work.join("songs"));
    let data = local.join("cat-smf-editor");
    for dir in [&data, &temp, &songs] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let song = songs.join(name);
    std::fs::copy(source, &song).unwrap();
    std::fs::write(
        data.join("status.json"),
        serde_json::json!({
            "navigation": { "last_mid": song },
            "session": { "file": name.to_string_lossy(), "track": 2, "mbt": null,
                         "tick": null, "row": null, "note_mode": true },
            "program_target": "Auto",
        })
        .to_string(),
    )
    .unwrap();
    let logs = temp.join("cat-resident-log");
    let roots_file = data.join("cache/patch-roots-v1.json");
    let mut run = Run {
        editor: Command::new(env!("CARGO_BIN_EXE_cat-smf-editor"))
            .env("LOCALAPPDATA", &local)
            .env("TEMP", &temp)
            .env("TMP", &temp)
            .spawn()
            .unwrap(),
        player: None,
    };
    let title = name.to_string_lossy().into_owned();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let editor = wait_for(
            "the song in the editor title",
            Duration::from_secs(90),
            || window(|process, t| process == run.editor.id() && t.contains(&title)),
        );
        let errors_after_load = std::fs::read_to_string(data.join("error.log")).unwrap_or_default();
        let roots_written = roots_file.is_file();
        // The restored song starts playing; its plugins open on the first frames.
        std::thread::sleep(Duration::from_secs(3));
        press(editor.0, T);
        let player = wait_for("the player process", Duration::from_secs(30), || {
            child_of(run.editor.id())
        });
        run.player = Some(player);
        let browser = wait_for("the patch browser", Duration::from_secs(30), || {
            window(|process, t| process == player && t == "Browse patches")
        });
        wait_for(
            "the catalog and an idle player",
            Duration::from_secs(90),
            || {
                let state = last_state(&logs)?;
                (field(&state, "results=", " status=")? != "0" && idle(&state)).then_some(())
            },
        );
        std::thread::sleep(Duration::from_secs(1));
        press(browser.0, DOWN);
        let applied = wait_for("the chosen patch to apply", Duration::from_secs(30), || {
            let state = last_state(&logs).filter(|state| idle(state))?;
            let applied = field(&state, "applied=", " offered=")?;
            (applied != "-").then(|| applied.to_owned())
        });
        press(browser.0, ESCAPE);
        std::thread::sleep(Duration::from_secs(2));
        (errors_after_load, roots_written, applied)
    }));
    run.close();
    println!("{}", timeline(&logs));
    println!("work directory: {}", work.display());
    let (errors_after_load, roots_written, applied) = match result {
        Ok(outcome) => outcome,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    assert!(
        !errors_after_load.contains("[load]"),
        "the song did not load: {errors_after_load}"
    );
    assert!(roots_written, "{} was not written", roots_file.display());

    let bytes = std::fs::read(&song).unwrap();
    let original = std::fs::read(source).unwrap();
    assert_ne!(bytes, original, "the song was not saved");
    let music = cat_smf_editor::smf_io::load(&song).unwrap();
    let owned: Vec<&[u8]> = music
        .tracks
        .iter()
        .flat_map(|track| &track.events)
        .map(|event| event.data.as_slice())
        .filter(|data| data.starts_with(b"}cat-plugin-player\0"))
        .collect();
    assert!(owned.len() >= 2, "no patch references were saved");
    for data in &owned {
        println!("saved: {}", String::from_utf8_lossy(data));
    }
    for absolute in [&b":\\"[..], b":/", b"bundle_path"] {
        assert!(
            !owned
                .iter()
                .any(|d| d.windows(absolute.len()).any(|w| w == absolute)),
            "a saved reference contains {:?}",
            String::from_utf8_lossy(absolute)
        );
    }
    let mut roots: Table = serde_json::from_slice(&std::fs::read(&roots_file).unwrap()).unwrap();
    cat_smf_editor::master_references::read(&music, &mut roots).unwrap();
    let mut displays = Vec::new();
    for track in &music.tracks {
        let settings = cat_smf_editor::instrument_layers::parse(track).unwrap();
        settings.validate_portable().unwrap();
        cat_smf_editor::instrument_layers::read(track, &mut roots).unwrap();
        displays.extend(
            settings
                .layers
                .into_iter()
                .filter_map(|l| Some(l.patch?.display)),
        );
    }
    println!("chosen: {applied}; saved layers: {displays:?}");
    assert!(
        displays.contains(&applied),
        "the chosen patch {applied:?} is not in the saved song: {displays:?}"
    );
    let _ = std::fs::remove_dir_all(&work);
}
