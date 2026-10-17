//! Opt-in real-plugin buffer verification. Never connects a cpal output stream.
use super::*;
use cat_smf_editor::{
    instrument_layers::{Layer, Settings as Saved},
    patch_reference::PatchReference,
    score::state_cache::Cache,
};

#[test]
#[ignore = "requires installed Surge XT, STATE_REQUEST_REFERENCE, STATE_REQUEST_WORK_DIR and RESIDENT_PLAYER; creates native plugin instances"]
fn native_duplicate_layers_sound_and_stopping_one_track_keeps_the_other_held() {
    let reference: PatchReference = serde_json::from_slice(
        &std::fs::read(std::env::var_os("STATE_REQUEST_REFERENCE").unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(reference.plugin.id, "org.surge-synth-team.surge-xt");
    let work = std::path::PathBuf::from(std::env::var_os("STATE_REQUEST_WORK_DIR").unwrap());
    let _library = cat_smf_editor::native_library::load_at(&work.join("native")).unwrap();
    let mut cache = Cache::new(Some(work));
    let player = Some(std::path::PathBuf::from(
        std::env::var_os("RESIDENT_PLAYER").unwrap(),
    ));
    let saved = Saved {
        layers: vec![
            Layer {
                patch: Some(reference.clone()),
                volume_db: -6.0206,
                ..Default::default()
            },
            Layer {
                id: 2,
                patch: Some(reference),
                octave: 1,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let patch = Patch::resolve(saved, |r| {
        cache.resolve(r, |r| {
            crate::player::state_request::generate(r, player.clone())
        })
    })
    .unwrap();
    let started = std::time::Instant::now();
    let track_a = Native::open(48000, &patch).unwrap();
    let track_b = Native::open(48000, &patch).unwrap();
    assert_eq!(track_a.layers.len(), 2);
    assert_eq!(track_b.layers.len(), 2);
    let mut a = unsafe { track_a.render() };
    let mut b = unsafe { track_b.render() };
    let mut velocities = [0; 128];
    velocities[60] = 80;
    let mut durations = [0; 128];
    durations[60] = u64::MAX;
    let mut audio_a = [0.0; 2048];
    let mut audio_b = [0.0; 2048];
    let mut heard = 0.0f32;
    for block in 0..64 {
        let changed = i32::from(block == 0);
        assert!(a.process(&velocities, &durations, changed, &mut audio_a, 2));
        assert!(b.process(&velocities, &durations, changed, &mut audio_b, 2));
        heard = heard.max(audio_a.iter().copied().map(f32::abs).fold(0.0, f32::max));
        track_a.pump();
        track_b.pump();
    }
    assert!(heard > 0.00001, "Real duplicate layers must produce signal");
    assert!(a.process(&[0; 128], &[0; 128], 3, &mut audio_a, 2));
    assert!(track_a.silence(&mut audio_a));
    for _ in 0..256 {
        assert!(a.process(&[0; 128], &[0; 128], 0, &mut audio_a, 2));
        assert!(b.process(&velocities, &durations, 0, &mut audio_b, 2));
        track_a.pump();
        track_b.pump();
    }
    let peak = |samples: &[f32]| samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
    assert!(
        peak(&audio_b) > 0.00001,
        "Stopping one track must leave the other held"
    );
    assert!(
        peak(&audio_a) < peak(&audio_b) * 0.1,
        "Stopped track must release its organ voices"
    );
    println!("Offline native layers: initial peak={heard}, stopped peak={}, held other track peak={}, elapsed={:?}; no output device used",peak(&audio_a),peak(&audio_b),started.elapsed());
    assert!(b.process(&[0; 128], &[0; 128], 3, &mut audio_b, 2));
    assert!(track_b.silence(&mut audio_b));
}

/// Both fixture effects write silence; bypassing passes the input through.
fn fixture_effects(bypass: [bool; 2]) -> Vec<cat_smf_editor::chain_settings::PluginState> {
    use cat_smf_editor::patch_reference::Source;
    let bundle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/shim/Release/clap_state_test_plugin.clap");
    assert!(bundle.is_file(), "build clap_state_test_plugin first");
    ["cat.test.state.effect", "cat.test.state.effect2"]
        .into_iter()
        .zip(bypass)
        .map(|(id, bypass)| {
            let plugin = cat_smf_editor::host::PluginKey {
                format: "CLAP".into(),
                id: id.into(),
                name: id.into(),
                vendor: "cat test".into(),
                bundle_path: bundle.to_string_lossy().into_owned(),
            };
            cat_smf_editor::chain_settings::PluginState {
                reference: Some(PatchReference {
                    plugin: plugin.clone(),
                    source: Source::Effect {
                        path: bundle.clone(),
                        value: String::new(),
                    },
                    display: id.into(),
                }),
                plugin,
                state: vec![1],
                preset_name: None,
                bypass,
            }
        })
        .collect()
}

#[test]
#[ignore = "requires installed Surge XT and the clap_state_test_plugin fixture in target/shim; creates native plugin instances, no audio output"]
fn native_effect_bypass_switches_running_track_and_master_chains() {
    let cache = cat_smf_editor::storage_paths::cache_directory().unwrap();
    let _library = cat_smf_editor::native_library::load_at(&cache).unwrap();
    let peak = |samples: &[f32]| samples.iter().copied().map(f32::abs).fold(0.0, f32::max);

    let mut patch = Patch {
        effects: fixture_effects([true, true]),
        ..Default::default()
    };
    let mut track = Native::open(48000, &patch).unwrap();
    let mut render = unsafe { track.render() };
    let mut velocities = [0; 128];
    velocities[60] = 100;
    let mut durations = [0; 128];
    durations[60] = u64::MAX;
    let mut audio = [0.0; 2048];
    let mut changed = 1;
    let mut track_peak = |render: &mut super::super::layer_render::Render, track: &Native| {
        let mut heard = 0.0f32;
        for _ in 0..16 {
            assert!(render.process(&velocities, &durations, changed, &mut audio, 2));
            changed = 0;
            track.pump();
            heard = peak(&audio);
        }
        heard
    };
    let mut track_peaks = vec![track_peak(&mut render, &track)];
    for bypass in [[false, true], [true, false], [true, true]] {
        patch.effects = fixture_effects(bypass);
        track.set_bypass(&patch).unwrap();
        track_peaks.push(track_peak(&mut render, &track));
    }
    println!("track peaks (both bypassed, first on, second on, both bypassed): {track_peaks:?}");
    assert!(
        track_peaks[0] > 0.00001,
        "Bypassed effects must pass the instrument"
    );
    assert_eq!(track_peaks[1], 0.0, "Enabled first effect must apply");
    assert_eq!(track_peaks[2], 0.0, "Enabled second effect must apply");
    assert!(
        track_peaks[3] > 0.00001,
        "Bypassing again must pass the instrument"
    );
    let mut reordered = patch.clone();
    reordered.effects.reverse();
    assert!(track.set_bypass(&reordered).is_err());
    drop(render);
    drop(track);

    let master = cat_smf_editor::chain::Chain::open_master(
        48000,
        &cat_smf_editor::master_effects::Settings {
            effects: fixture_effects([true, true]),
            ..Default::default()
        },
    )
    .unwrap();
    let mut render = unsafe { master.audio_ref() };
    let master_out = |render: &mut cat_smf_editor::host::ProcessorRef| {
        let mut input = [0.25; 2048];
        for _ in 0..4 {
            input.fill(0.25);
            assert!(render.process_input(&mut input, 2));
            master.pump();
        }
        peak(&input)
    };
    let mut master_peaks = vec![master_out(&mut render)];
    for bypass in [[false, true], [true, false], [true, true]] {
        master.set_master_bypass(&fixture_effects(bypass)).unwrap();
        master_peaks.push(master_out(&mut render));
    }
    println!("master peaks (both bypassed, first on, second on, both bypassed): {master_peaks:?}");
    assert_eq!(master_peaks, [0.25, 0.0, 0.0, 0.25]);
}
