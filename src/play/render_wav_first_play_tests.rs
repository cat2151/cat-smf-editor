//! Opt-in check that the playback started at launch sounds a track's first note as a
//! later playback does. It loads the song and starts playback as `App::bootstrap` does,
//! stops, and plays again, recording the default audio device's output. Other tracks
//! play at zero gain, so the recording is the track alone.
//!
//! Set AUDIO_PROBE_SMF to an existing SMF and AUDIO_PROBE_TRACK to a 1-based track, then run:
//! cargo test --release --bin cat-smf-editor first_play_sounds_like_second -- --ignored --nocapture --test-threads=1
//! Optional AUDIO_PROBE_SECONDS (3) of each playback; AUDIO_PROBE_WAV_DIR receives
//! `first.wav` and `second.wav`.
use super::*;
use crate::play::analysis::recorder::Recorder;
use std::time::{Duration, Instant};

const THRESHOLD: f32 = 1e-3;
const TOLERANCE_SECONDS: f64 = 0.050;
/// The span from the first sound whose peak stands for the first hit's attack.
const ATTACK_SECONDS: f64 = 0.030;
/// The least share of the later playback's attack the first playback must reach.
const ATTACK_RATIO: f32 = 0.8;

struct Heard {
    /// Seconds from the start of the stream that plays it to the first sound.
    onset: Option<f64>,
    attack: f32,
    /// How long the start request took to reach the first callback.
    starting: Duration,
    /// The recording from the stream's start on.
    frames: Vec<[f32; 2]>,
}

/// Starts as `App::bootstrap` does, then plays on with GUI-rate servicing.
fn play(app: &mut App, recorder: &mut Recorder, seconds: f64) -> Heard {
    recorder.drain();
    let requested_at = recorder.frames.len();
    let streams = recorder.starts.len();
    let requested = Instant::now();
    app.start_initial_playback();
    crate::play::transport::advance(app);
    while crate::play::transport::preparing(app) && requested.elapsed() < Duration::from_secs(10) {
        app.audition.pump();
        std::thread::sleep(Duration::from_millis(1));
    }
    let starting = requested.elapsed();
    assert!(app.audition.error().is_none(), "{:?}", app.audition.error());
    app.audition.pump();
    while requested.elapsed().as_secs_f64() < seconds {
        crate::play::transport::advance(app);
        app.audition.pump();
        recorder.drain();
        std::thread::sleep(Duration::from_millis(5));
    }
    crate::play::transport::stop(app);
    recorder.drain();
    let rate = f64::from(recorder.rate);
    // A playback that opens a stream starts with it; one that reuses the stream, at the request.
    let from = recorder.starts[streams..]
        .last()
        .copied()
        .unwrap_or(requested_at);
    let heard = &recorder.frames[from..];
    let onset = heard
        .iter()
        .position(|frame| frame.iter().any(|s| s.abs() > THRESHOLD));
    Heard {
        onset: onset.map(|frame| frame as f64 / rate),
        attack: onset.map_or(0.0, |onset| {
            heard[onset..(onset + (ATTACK_SECONDS * rate) as usize).min(heard.len())]
                .iter()
                .flatten()
                .fold(0.0f32, |m, s| m.max(s.abs()))
        }),
        starting,
        frames: heard.to_vec(),
    }
}

#[test]
#[ignore = "requires AUDIO_PROBE_SMF, AUDIO_PROBE_TRACK and installed plugins; plays through the default audio device"]
fn first_play_sounds_like_second() {
    let mid = std::env::var_os("AUDIO_PROBE_SMF").expect("explicit SMF path");
    let track: usize = std::env::var("AUDIO_PROBE_TRACK")
        .expect("explicit 1-based track")
        .parse::<usize>()
        .unwrap()
        - 1;
    let seconds: f64 = std::env::var("AUDIO_PROBE_SECONDS")
        .ok()
        .map_or(3.0, |s| s.parse().unwrap());
    let work = tempfile::tempdir().unwrap();
    let _dll = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let table = serde_json::from_slice(
        &std::fs::read(cat_smf_editor::storage_paths::cache(&root).join("patch-roots-v1.json"))
            .unwrap(),
    )
    .unwrap();
    // Tests resolve patch roots from `Assumed`; this one needs the machine's real table.
    let _roots = crate::player::patch_roots::Assumed::with_table(table);
    // As `App::bootstrap` loads, without a status file to restore or save.
    let mut app = App::empty();
    app.startup.pending = true;
    app.open_initial(Some(mid.into()));
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let tracks = app.loaded.as_ref().unwrap().music.tracks.len();
    for other in (0..tracks).filter(|&t| t != track) {
        app.audition.set_gain(other, 0.0);
    }
    let first_note = crate::play::transport::prepare(&app, Mode::AllTracks)
        .unwrap()
        .parts
        .iter()
        .filter(|(t, _, _)| *t == track)
        .flat_map(|(_, _, sequence)| sequence.notes.iter().map(|n| n.start))
        .fold(f64::INFINITY, f64::min);
    assert!(first_note.is_finite(), "track {} has no notes", track + 1);
    let mut recorder = Recorder::start();
    let first = play(&mut app, &mut recorder, seconds);
    let rest = Instant::now();
    while rest.elapsed() < Duration::from_secs(1) {
        app.audition.pump();
        recorder.drain();
        std::thread::sleep(Duration::from_millis(5));
    }
    let second = play(&mut app, &mut recorder, seconds);
    for (name, heard) in [("first", &first), ("second", &second)] {
        eprintln!(
            "{name}: track={} starting={:?} first_note={first_note:.3}s onset={:?} attack={:.3}",
            track + 1,
            heard.starting,
            heard.onset,
            heard.attack,
        );
    }
    if let Some(dir) = std::env::var_os("AUDIO_PROBE_WAV_DIR") {
        for (name, heard) in [("first", &first), ("second", &second)] {
            let samples: Vec<f32> = heard.frames.iter().flatten().copied().collect();
            crate::play::wav::write(
                &std::path::Path::new(&dir).join(format!("{name}.wav")),
                recorder.rate,
                &samples,
            )
            .unwrap();
        }
    }
    let (Some(one), Some(two)) = (first.onset, second.onset) else {
        panic!("a playback never sounds");
    };
    assert!(
        (one - two).abs() <= TOLERANCE_SECONDS,
        "first playback sounds at {one:.3}s, second at {two:.3}s"
    );
    assert!(
        first.attack >= second.attack * ATTACK_RATIO,
        "first attack peaks at {:.3}, second at {:.3}",
        first.attack,
        second.attack
    );
}
