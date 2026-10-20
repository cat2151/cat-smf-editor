//! 実 engine で歌わせた WAV と、editor の描いた WAV を sample 単位で比べる。repo root の
//! `.voicevox-engine-path`（git 管理外）に `run.exe` のパスを1行で書き、
//! `cargo test -- --ignored voicevox` で走らせる。
use super::prerender;
use super::tests::{app_at, donguri, render, BAR_2, FADE, RATE, SINGER};
use crate::play::playback::Mode;
use cat_smf_editor::voicevox::{cache, client::Client, pcm, score::from_track};
use std::time::{Duration, Instant};

/// lib の engine test と並列に走っても互いの engine に触らない port。
const PORT: u16 = 50125;
/// 量子化 1 段ぶんの2倍まで。
const TOLERANCE: f32 = 2.0 / 32768.0;
const LEAD: usize = 15 * 512;
const QUARTER: usize = 45 * 512;

fn configured() -> std::path::PathBuf {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/.voicevox-engine-path");
    std::fs::read_to_string(file)
        .expect(".voicevox-engine-path")
        .trim()
        .into()
}

fn running(pid: u32) -> bool {
    let tasks = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&tasks.stdout).contains(&format!("\"{pid}\""))
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_sung_track_renders_the_engine_wav_from_the_head_and_from_bar_2() {
    let client = Client::new(&format!("http://127.0.0.1:{PORT}"));
    assert!(client.version().is_err(), "port {PORT} is already in use");
    let cache_root = tempfile::tempdir().unwrap();
    let score = from_track(&donguri(), SINGER).unwrap();
    // The editor's own path: playback waits while the engine starts and sings in the background.
    let mut app = app_at(donguri(), None);
    app.config.voicevox_engine = Some(configured());
    app.voicevox.prerender.target = Some((PORT, cache_root.path().into()));
    crate::play::playback::start(&mut app, Mode::AllTracks);
    let started = Instant::now();
    while app.playback.pending.is_some() {
        assert!(started.elapsed() < Duration::from_secs(90));
        crate::play::transport::advance(&mut app);
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("engine start and singing: {:?}", started.elapsed());
    assert!(app.playback.active.is_some());
    assert_eq!(app.voicevox.error, None);
    let pid = app
        .voicevox
        .engine
        .as_ref()
        .and_then(|engine| engine.started_pid())
        .expect("started by the editor");
    let file = cache_root
        .path()
        .join(format!("{}.wav", cache::key(&score, RATE)));
    let (rate, wav) = pcm::decode(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(rate, RATE);
    assert_eq!(wav.len(), score.total_frames() as usize * 512 * 2);
    prerender::shut_down(&mut app);
    assert!(!running(pid), "engine {pid} is still running");

    let mut head_app = app_at(donguri(), None);
    prerender::insert(&mut head_app, &score, RATE, wav.clone());
    let head = render(&mut head_app, Mode::AllTracks);
    let body = head.len() - FADE * 2;
    let expected = &wav[LEAD * 2..LEAD * 2 + body];
    let loud = expected.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(loud > 0.05, "the engine sang silence ({loud})");
    let head_error = worst(&head[..body], expected);
    println!(
        "head: {} frames, peak {loud}, worst |diff| {head_error}",
        body / 2
    );
    assert!(head_error <= TOLERANCE);

    let mut bar_app = app_at(donguri(), Some(BAR_2 + 10));
    prerender::insert(&mut bar_app, &score, RATE, wav.clone());
    let from_bar = render(&mut bar_app, Mode::AllTracksFromMeasure);
    // ぐ was sounding at the bar line; it restarts from its head until り one quarter later.
    let bar = 4 * QUARTER * 2;
    let held = QUARTER * 2;
    let body = from_bar.len() - FADE * 2;
    let after_error = worst(&from_bar[held..body], &head[bar + held..bar + body]);
    let held_error = worst(&from_bar[..held], &head[bar..bar + held]);
    println!("from bar 2: worst |diff| after the held note {after_error}, within it {held_error}");
    assert!(after_error <= TOLERANCE);
    assert!(
        held_error > 0.01,
        "the held note should restart from its head"
    );
}
