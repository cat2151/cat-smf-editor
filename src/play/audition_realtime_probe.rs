//! Opt-in replay of an existing SMF and cached patches, with no GUI or saves.
//!
//! Set AUDIO_PROBE_SMF to an existing SMF, then run:
//! cargo test --release --bin cat-smf-editor realtime_song_probe -- --ignored --nocapture --test-threads=1
//! Optional AUDIO_PROBE_SECONDS (15). Dropouts go to `error.log` as `audio-dropout` lines
//! carrying the pid printed at start.
//!
//! `realtime_live_probe` opens the song's patches as the Virtual MIDI keyboard's stream
//! and wakes it with three keys after rests.
//!
//! `offline_song_render` writes the same song through the offline mixer to AUDIO_PROBE_WAV,
//! so two builds of the mixer can be compared sample by sample.
use super::*;
use std::time::{Duration, Instant};

/// The song with its patches, master chain and track gains resolved from the cache.
/// The native library must already be loaded.
fn song(path: &std::ffi::OsStr) -> (crate::app::App, cat_smf_editor::master_effects::Settings) {
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let music = cat_smf_editor::smf_io::load(std::path::Path::new(path)).unwrap();
    let mut roots: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let mut cache = cat_smf_editor::score::state_cache::Cache::new(Some(root));
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let loaded = app.loaded.as_mut().unwrap();
    loaded.patches = vec![Patch::default(); music.tracks.len()];
    loaded.music = music;
    loaded.refresh();
    for track in 0..loaded.music.tracks.len() {
        let saved =
            cat_smf_editor::instrument_layers::read(&loaded.music.tracks[track], &mut roots)
                .unwrap();
        loaded.patches[track] = Patch::resolve(saved, |reference| {
            cache.resolve(reference, |_| {
                Err("Probe refuses to replace cached patch state".into())
            })
        })
        .unwrap();
        loaded.patches[track].drums =
            cat_smf_editor::score::drums::read(&loaded.music.tracks[track]).unwrap();
    }
    let saved_master = cat_smf_editor::master_references::read(&loaded.music, &mut roots).unwrap();
    let master = cat_smf_editor::master_effects::Settings {
        effects: saved_master
            .effects
            .iter()
            .map(|effect| {
                cat_smf_editor::score::layer_track::resolve_effect(effect, |reference| {
                    cache.resolve(reference, |_| {
                        Err("Probe refuses to replace cached master state".into())
                    })
                })
            })
            .collect::<Result<_, _>>()
            .unwrap(),
        ..Default::default()
    };
    (app, master)
}

fn gains(app: &crate::app::App) -> Vec<f32> {
    app.loaded
        .as_ref()
        .unwrap()
        .music
        .tracks
        .iter()
        .map(|track| crate::play::track_volume::read(track).unwrap())
        .collect()
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF and installed plugins; plays through the default audio device"]
fn realtime_song_probe() {
    let path = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let seconds: u64 = std::env::var("AUDIO_PROBE_SECONDS")
        .ok()
        .map_or(15, |s| s.parse().unwrap());
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let (app, master) = song(&path);
    let prepared =
        crate::play::transport::prepare(&app, crate::play::playback::Mode::AllTracks).unwrap();
    // Automation owners with their curve counts; one owner splits every part at the 64-frame grid.
    eprintln!(
        "pid={} parts={:?} automated={:?}",
        std::process::id(),
        prepared
            .parts
            .iter()
            .map(|(t, p, s)| (t + 1, p.layers.len(), p.effects.len(), s.notes.len()))
            .collect::<Vec<_>>(),
        prepared
            .active
            .automation
            .owners
            .iter()
            .map(|plan| (plan.owner + 1, plan.curves.len()))
            .collect::<Vec<_>>()
    );
    let mut audition = Audition::default();
    audition.set_master(master);
    for (track, gain) in gains(&app).into_iter().enumerate() {
        audition.set_gain(track, gain);
    }
    assert!(
        audition.sequences_automated(
            prepared.parts,
            vec![],
            &prepared.active.automation,
            prepared.active.start_seconds
        ),
        "{:?}",
        audition.error()
    );
    let voice = audition.voice.as_mut().unwrap();
    let start = Instant::now();
    let mut report = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        // Same main-thread native servicing as the GUI, without file saves or painting.
        voice
            .pump(
                cat_smf_editor::host::Host::main_thread_work_pending(),
                || {},
            )
            .unwrap();
        assert!(!voice.failed.load(Ordering::Relaxed));
        if report.elapsed() >= Duration::from_secs(1) {
            voice.poll_diagnostics();
            report = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // The last dropout is only recorded once its 30-callback window closes.
    std::thread::sleep(Duration::from_millis(1100));
    voice.poll_diagnostics();
    eprintln!("pid={} done", std::process::id());
}

/// Services the voice's native work and reports diagnostics once a second for `duration`.
fn serve(voice: &mut Voice, duration: Duration, report: &mut Instant) {
    let start = Instant::now();
    while start.elapsed() < duration {
        voice
            .pump(
                cat_smf_editor::host::Host::main_thread_work_pending(),
                || {},
            )
            .unwrap();
        assert!(!voice.failed.load(Ordering::Relaxed));
        if report.elapsed() >= Duration::from_secs(1) {
            voice.poll_diagnostics();
            *report = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The Virtual MIDI keyboard's stream with the song's patches: it rests, then three keys
/// each wake it. Each key is preceded by an `audio-probe-key` line in `error.log`, and each
/// rest by an `audio-rest` line.
#[test]
#[ignore = "requires AUDIO_PROBE_SMF and installed plugins; plays through the default audio device"]
fn realtime_live_probe() {
    let path = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let (app, master) = song(&path);
    let prepared =
        crate::play::transport::prepare(&app, crate::play::playback::Mode::AllTracks).unwrap();
    let sounds: Vec<_> = prepared
        .parts
        .iter()
        .map(|(track, patch, _)| (*track, patch.clone(), live::Live::default()))
        .collect();
    eprintln!("pid={} parts={}", std::process::id(), sounds.len());
    let mut audition = Audition::default();
    audition.set_master(master);
    for (track, gain) in gains(&app).into_iter().enumerate() {
        audition.set_gain(track, gain);
    }
    let click = Arc::new(click::Click::new());
    assert!(
        audition.start_live(&sounds, &click),
        "{:?}",
        audition.error()
    );
    let voice = audition
        .live_voice
        .as_mut()
        .and_then(|v| v.voice.as_mut())
        .unwrap();
    let queue = &sounds[0].2.queue;
    let mut report = Instant::now();
    for key in 0..3 {
        // Long enough for a release and reverb tail plus the second of silence before a rest.
        serve(voice, Duration::from_secs(12), &mut report);
        crate::store::error_log::write(
            "audio-probe-key",
            &format!(
                "pid={} stream={} key={key}",
                std::process::id(),
                voice.diagnostic_id()
            ),
        );
        live::push(queue, &[60], 100);
        serve(voice, Duration::from_secs(1), &mut report);
        live::push(queue, &[60], 0);
    }
    serve(voice, Duration::from_secs(3), &mut report);
    // The last dropout is only recorded once its 30-callback window closes.
    std::thread::sleep(Duration::from_millis(1100));
    voice.poll_diagnostics();
    eprintln!("pid={} done", std::process::id());
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF, AUDIO_PROBE_WAV and installed plugins"]
fn offline_song_render() {
    let path = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let wav = std::env::var_os("AUDIO_PROBE_WAV").expect("explicit WAV path");
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let (app, master) = song(&path);
    let prepared =
        crate::play::transport::prepare(&app, crate::play::playback::Mode::AllTracks).unwrap();
    let rate = 48000;
    let instruments = offline::Instruments::open_automated(
        rate,
        &prepared.parts,
        &master,
        &prepared.active.automation,
    )
    .unwrap();
    // SAFETY: the render thread is joined before the instruments drop.
    let job = unsafe {
        instruments.job_at(
            rate,
            prepared.parts,
            &gains(&app),
            true,
            vec![],
            prepared.active.start_seconds,
        )
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let samples = std::thread::scope(|scope| {
        let render = scope.spawn(|| job.run(&cancel));
        while !render.is_finished() {
            instruments.pump();
            std::thread::sleep(Duration::from_millis(2));
        }
        render.join().unwrap().unwrap().unwrap()
    });
    crate::play::wav::write(std::path::Path::new(&wav), rate, &samples).unwrap();
    eprintln!("frames={}", samples.len() / offline::CHANNELS);
}
