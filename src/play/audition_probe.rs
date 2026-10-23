//! Opt-in replay of an existing SMF and cached patches, with no audio device or saves.
use super::*;
use crate::play::{audition_native::Native, event_timeline::Timeline};
use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "requires AUDIO_PROBE_SMF and existing patch caches; renders offline, never opens an audio device"]
fn cached_song_stop_probe() {
    let path = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let music = cat_smf_editor::smf_io::load(std::path::Path::new(&path)).unwrap();
    let mut roots: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let mut cache = cat_smf_editor::score::state_cache::Cache::new(Some(root));
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let loaded = app.loaded.as_mut().unwrap();
    loaded.patches = vec![Patch::default(); music.tracks.len()];
    loaded.music = music;
    loaded.refresh();
    let track_count = app.loaded.as_ref().unwrap().music.tracks.len();
    for track in 0..track_count {
        if std::env::var("AUDIO_PROBE_TRACK")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .is_some_and(|t| t != track + 1)
        {
            continue;
        }
        app.track = track;
        let sequence =
            crate::play::playback::sequence(&app, crate::play::playback::Mode::Track).unwrap();
        if sequence.notes.is_empty() {
            continue;
        }
        let saved = cat_smf_editor::instrument_layers::read(
            &app.loaded.as_ref().unwrap().music.tracks[track],
            &mut roots,
        )
        .unwrap();
        let patch = Patch::resolve(saved, |reference| {
            cache.resolve(reference, |_| {
                Err("Probe refuses to generate or replace cached patch state".into())
            })
        })
        .unwrap();
        eprintln!(
            "track={} notes={} events={} first={:?} patches={:?}",
            track + 1,
            sequence.notes.len(),
            sequence.events.len(),
            sequence.events.iter().take(8).collect::<Vec<_>>(),
            patch
                .layers
                .iter()
                .map(|l| l.instrument.as_ref().and_then(|p| p.preset_name.as_ref()))
                .collect::<Vec<_>>()
        );
        for canonical in [false, true] {
            let mut sequence = sequence.clone();
            if !canonical {
                // Deliberately reintroduce the old encoding as the A/B control.
                for event in &mut sequence.events {
                    if event.word >> 20 & 15 == 8 && event.word & 127 == 0 {
                        event.word = (event.word & !0x00f0_0000) | 0x0090_0000;
                    }
                }
            }
            let native = Native::open(48000, &patch).unwrap();
            let renders = vec![unsafe { native.render() }];
            let mixer = mix::Mixer::new(48000, 2, 1, None);
            let command = Command {
                generation: 1,
                parts: Arc::new(vec![Part {
                    timeline: Some(Arc::new(Timeline::new(&sequence, 0.0, 48000))),
                    sequence: Some(Arc::new(sequence.clone())),
                    ..Part::default()
                }]),
                auto_gain_disabled: true,
                ..Command::default()
            };
            let measured = std::thread::scope(|scope| {
                let worker = scope.spawn(move || render(mixer, renders, command));
                while !worker.is_finished() {
                    native.pump();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                worker.join().unwrap()
            });
            for (second, (peak, rms)) in measured.iter().enumerate() {
                eprintln!(
                    "track={} canonical={canonical} second={second} peak={peak:.6} rms={rms:.6}",
                    track + 1
                );
            }
            if canonical && let Ok(limit) = std::env::var("AUDIO_PROBE_MAX_TAIL") {
                let limit: f32 = limit.parse().unwrap();
                assert!(
                    measured.last().unwrap().0 < limit,
                    "tail did not reach {limit}"
                );
            }
        }
    }
}

fn render(
    mut mixer: mix::Mixer,
    mut renders: Vec<crate::play::layer_render::Render>,
    mut command: Command,
) -> Vec<(f32, f64)> {
    let failed = AtomicBool::new(false);
    let mut measured = Vec::new();
    for second in 0..10 {
        if second == 2 {
            command.generation += 1;
            command.parts = Arc::new(vec![Part::default()]);
        }
        let mut frames = 0;
        let mut peak = 0.0_f32;
        let mut square = 0.0_f64;
        while frames < 48000 {
            let samples = mixer.block(&mut renders, &command, (48000 - frames).min(256), &failed);
            for &sample in samples {
                peak = peak.max(sample.abs());
                square += f64::from(sample).powi(2);
            }
            frames += samples.len() / 2;
            assert!(!failed.load(Ordering::Relaxed));
        }
        measured.push((peak, (square / 96000.0).sqrt()));
    }
    measured
}
