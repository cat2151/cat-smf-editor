//! Manual full/minimal-data-home comparison, in separate processes, without a device.
use crate::{host::PluginKey, note_render::Render};
use serde::Deserialize;
mod instance;
use instance::Instance;

#[derive(Deserialize)]
struct Cached {
    reference: crate::patch_reference::PatchReference,
    state: Vec<u8>,
    #[serde(default)]
    fxp_state: Option<Vec<u8>>,
}

#[test]
#[ignore = "requires installed Surge XT and SURGE_AUDIO_PROBE_INPUT/OUTPUT directories; offline only"]
fn surge_data_home_audio_probe() {
    let input = std::path::PathBuf::from(std::env::var_os("SURGE_AUDIO_PROBE_INPUT").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("SURGE_AUDIO_PROBE_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let _library =
        crate::native_library::load_at(&crate::storage_paths::cache_directory().unwrap()).unwrap();
    let mut patches: Vec<(String, PluginKey, Option<Vec<u8>>)> = vec![(
        "default".into(),
        crate::chain::default_instrument().unwrap(),
        None,
    )];
    for path in std::fs::read_dir(input).unwrap() {
        let path = path.unwrap().path();
        let cached: Cached = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(matches!(
            cached.reference.plugin.id.as_str(),
            "org.surge-synth-team.surge-xt" | "org.surge-synth-team.surge-xt-fx"
        ));
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if let Some(state) = cached.fxp_state {
            patches.push((
                format!("{name}-fxp"),
                cached.reference.plugin.clone(),
                Some(state),
            ));
        }
        patches.push((name, cached.reference.plugin, Some(cached.state)));
    }
    let repeats = std::env::var("SURGE_AUDIO_PROBE_REPEATS")
        .ok()
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(3);
    for (name, key, state) in patches {
        let effect = key.id.ends_with("-fx");
        for repeat in 0..repeats {
            let chain = Instance::open(&key, state.as_deref(), effect);
            std::fs::write(output.join(format!("{name}-{repeat}.state")), chain.save()).unwrap();
            let mut render = Render::new(chain.audio());
            let mut effect_render = chain.audio();
            let mut velocity = [0; 128];
            let mut duration = [0; 128];
            for pitch in [48, 60, 67] {
                velocity[pitch] = 90;
                duration[pitch] = 96000;
            }
            let mut samples = Vec::new();
            for block in 0..128 {
                let mut out = [0.0; 2048];
                if effect {
                    for (frame, pair) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                        let t = (block * 1024 + frame) as f32 / 48000.0;
                        pair.fill(0.25 * (t * std::f32::consts::TAU * 220.0).sin());
                    }
                    assert!(effect_render.process_input(&mut out, 2));
                } else {
                    assert!(render.process(
                        &velocity,
                        &duration,
                        i32::from(block == 0),
                        &mut out,
                        2
                    ));
                }
                assert!(out.iter().all(|s| s.is_finite()));
                samples.extend_from_slice(&out);
                chain.pump();
            }
            let rms = (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>()
                / samples.len() as f64)
                .sqrt();
            assert!(rms > 0.00001, "{name} must produce sound");
            println!("{name} repeat={repeat} RMS={rms}");
            let bytes: Vec<_> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            std::fs::write(output.join(format!("{name}-{repeat}.f32")), bytes).unwrap();
        }
    }
}
