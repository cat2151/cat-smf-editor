//! Opt-in check that a rendered track sounds from its first note.
//!
//! Set AUDIO_PROBE_SMF to an existing SMF and AUDIO_PROBE_TRACK to a 1-based track, then run:
//! cargo test --release --bin cat-smf-editor render_wav_sounds_from_first_note -- --ignored --nocapture --test-threads=1
//! `render_wav_all_tracks_sounds_from_first_note` checks the same track inside all-track playback.
//!
//! `render_wav_load_probe` renders the same part with AUDIO_PROBE_WAIT_MS (0) of pumping
//! before the first block, at the device's speed when AUDIO_PROBE_PACED is 1, and prints
//! when the first sound was rendered. AUDIO_PROBE_NOPUMP=1 leaves the plugins unpumped.
use super::*;
use std::time::Instant;

/// Louder than plugin noise floors, far below any drum hit.
const THRESHOLD: f32 = 1e-3;
const TOLERANCE_SECONDS: f64 = 0.050;

struct Probe {
    _dll: libloading::Library,
    _roots: crate::player::patch_roots::ActualRoots,
    _work: tempfile::TempDir,
    app: App,
    prepared: Prepared,
    track: usize,
    first_note: f64,
}

/// `all` prepares all-track playback in place of the track's own.
fn probe(all: bool) -> Probe {
    let mid = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let track: usize = std::env::var("AUDIO_PROBE_TRACK")
        .expect("explicit 1-based track")
        .parse()
        .unwrap();
    let work = tempfile::tempdir().unwrap();
    let dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let table = serde_json::from_slice(
        &std::fs::read(cat_smf_editor::storage_paths::cache(&root).join("patch-roots-v1.json"))
            .unwrap(),
    )
    .unwrap();
    // Tests resolve patch roots from `Assumed`; this one needs the machine's real table.
    let roots = crate::player::patch_roots::Assumed::with_table(table);
    let mut app = load(None, mid.into()).unwrap();
    let prepared = prepare(&mut app, (!all).then_some(track - 1)).unwrap();
    let first_note = prepared
        .parts
        .iter()
        .filter(|(t, _, _)| *t == track - 1)
        .flat_map(|(_, _, sequence)| sequence.notes.iter().map(|n| n.start))
        .fold(f64::INFINITY, f64::min);
    assert!(first_note.is_finite(), "track {track} has no notes");
    Probe {
        _dll: dll,
        _roots: roots,
        _work: work,
        app,
        prepared,
        track,
        first_note,
    }
}

/// Seconds from the start to the first frame above `THRESHOLD`.
fn onset(samples: &[f32]) -> Option<f64> {
    samples
        .chunks(CHANNELS)
        .position(|frame| frame.iter().any(|s| s.abs() > THRESHOLD))
        .map(|frame| frame as f64 / f64::from(RATE))
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF, AUDIO_PROBE_TRACK and installed plugins"]
fn render_wav_sounds_from_first_note() {
    // Named bindings keep the library and patch roots alive until the end.
    let Probe {
        _dll,
        _roots,
        _work,
        app,
        prepared,
        track,
        first_note,
    } = probe(false);
    assert_sounds_from(&app, prepared, track, first_note);
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF, AUDIO_PROBE_TRACK and installed plugins"]
fn render_wav_all_tracks_sounds_from_first_note() {
    // Named bindings keep the library and patch roots alive until the end.
    let Probe {
        _dll,
        _roots,
        _work,
        mut app,
        prepared,
        track,
        first_note,
    } = probe(true);
    // Every track's plugins open and render as in the full mix; only this one is heard.
    let tracks = app.loaded.as_ref().unwrap().music.tracks.len();
    for other in (0..tracks).filter(|&t| t != track - 1) {
        app.audition.set_gain(other, 0.0);
    }
    assert_sounds_from(&app, prepared, track, first_note);
}

fn assert_sounds_from(app: &App, prepared: Prepared, track: usize, first_note: f64) {
    let samples = samples(app, prepared).unwrap();
    let onset = onset(&samples);
    // Peaks of the first 100ms in 10ms steps, to compare the first hit across renders.
    let envelope: Vec<String> = samples
        .chunks(CHANNELS * RATE as usize / 100)
        .take(10)
        .map(|c| format!("{:.2}", c.iter().fold(0.0f32, |m, s| m.max(s.abs()))))
        .collect();
    eprintln!("envelope={}", envelope.join(" "));
    eprintln!(
        "track={track} first_note={first_note:.3}s onset={onset:?} length={:.3}s peak={:.3}",
        (samples.len() / CHANNELS) as f64 / f64::from(RATE),
        samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    );
    let onset = onset.expect("the track never sounds");
    assert!(
        (onset - first_note).abs() <= TOLERANCE_SECONDS,
        "sound starts at {onset:.3}s, first note at {first_note:.3}s"
    );
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF, AUDIO_PROBE_TRACK and installed plugins"]
fn render_wav_load_probe() {
    let wait = std::time::Duration::from_millis(
        std::env::var("AUDIO_PROBE_WAIT_MS")
            .ok()
            .map_or(0, |s| s.parse().unwrap()),
    );
    let paced = std::env::var("AUDIO_PROBE_PACED").is_ok_and(|s| s == "1");
    let pumped = !std::env::var("AUDIO_PROBE_NOPUMP").is_ok_and(|s| s == "1");
    let rounds: usize = std::env::var("AUDIO_PROBE_ROUNDS")
        .ok()
        .map_or(1, |s| s.parse().unwrap());
    let effects = !std::env::var("AUDIO_PROBE_NO_EFFECTS").is_ok_and(|s| s == "1");
    // Named bindings keep the library and patch roots alive until the end.
    let Probe {
        _dll,
        _roots,
        _work,
        app,
        mut prepared,
        first_note,
        track: _,
    } = probe(false);
    for (t, patch, _) in &prepared.parts {
        eprintln!(
            "track={} layers={:?} effects={:?}",
            t + 1,
            patch
                .layers
                .iter()
                .map(|l| l.instrument.as_ref().map(|s| &s.plugin.id))
                .collect::<Vec<_>>(),
            patch
                .effects
                .iter()
                .map(|e| &e.plugin.id)
                .collect::<Vec<_>>()
        );
    }
    if !effects {
        for (_, patch, _) in &mut prepared.parts {
            patch.effects.clear();
        }
    }
    // The inputs of `samples`, with the timed run in place of `Job::run`.
    let music = &app.loaded.as_ref().unwrap().music;
    let takes = crate::play::sung::prerender::directory(&app).map_or_else(Vec::new, |d| {
        stored(
            music,
            &prepared.parts,
            prepared.active.start_seconds,
            RATE,
            &d,
        )
    });
    let gains: Vec<f32> = (0..music.tracks.len())
        .map(|t| app.audition.gain(t))
        .collect();
    let since = Instant::now();
    let instruments = Instruments::open_automated(
        RATE,
        &prepared.parts,
        app.audition.master(),
        &prepared.active.automation,
    )
    .unwrap();
    let opened = since.elapsed();
    for round in 0..rounds {
        if round > 0 {
            crate::play::audition::offline::load_probe::silence(&instruments);
        }
        // SAFETY: the render thread is joined before the instruments drop.
        let job = unsafe {
            instruments.job_at(
                RATE,
                prepared.parts.clone(),
                &gains,
                !app.audition.auto_gain_enabled(),
                takes.clone(),
                prepared.active.start_seconds,
            )
        };
        let (samples, stamps) = std::thread::scope(|scope| {
            let thread = scope.spawn(|| {
                crate::play::audition::offline::load_probe::run_timed(job, since, wait, paced)
            });
            while pumped && !thread.is_finished() {
                instruments.pump();
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            thread.join().unwrap()
        });
        // Peaks of the first 200ms in 10ms steps, to compare the first hit across runs.
        let envelope: Vec<String> = samples
            .chunks(CHANNELS * RATE as usize / 100)
            .take(20)
            .map(|c| format!("{:.2}", c.iter().fold(0.0f32, |m, s| m.max(s.abs()))))
            .collect();
        let onset = onset(&samples);
        let rendered_at = onset.map(|s| stamps[(s * f64::from(RATE)) as usize / 1024]);
        eprintln!(
            "round={round} effects={effects} wait={wait:?} paced={paced} pumped={pumped} \
             opened={opened:?} first_note={first_note:.3}s onset={onset:?} \
             onset_rendered_at={rendered_at:?} finished_at={:?}\nenvelope={}",
            stamps.last(),
            envelope.join(" ")
        );
    }
}
