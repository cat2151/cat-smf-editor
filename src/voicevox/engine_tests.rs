//! 実 engine との契約。repo root の `.voicevox-engine-path`（git 管理外）に `run.exe` の
//! パスを1行で書いておき、`cargo test -- --ignored voicevox` で走らせる。
use super::engine::{connect, executable, Engine, Failure, START_TIMEOUT};
use super::{cache, client::Client, score::Score, score::ScoreNote};
use std::process::Command;
use std::time::{Duration, Instant};

/// test ごとに別の port を使い、並列に走っても互いの engine に触らない。
const PORT: u16 = 50121;
const LIFECYCLE_PORT: u16 = 50123;
const ONE_NOTE_PORT: u16 = 50129;
const RATE: u32 = 48000;

fn configured() -> std::path::PathBuf {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/.voicevox-engine-path");
    let path = std::fs::read_to_string(file).expect(".voicevox-engine-path");
    path.trim().into()
}

/// port が空いていることを確かめてから、test 用の port に engine を起動する。
fn start(port: u16) -> Engine {
    let client = Client::new(&format!("http://127.0.0.1:{port}"));
    assert!(client.version().is_err(), "port {port} is already in use");
    let engine = connect(port, Some(&configured()), START_TIMEOUT).expect("start engine");
    assert!(engine.started_pid().is_some());
    engine
}

fn running(pid: u32) -> bool {
    let tasks = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&tasks.stdout).contains(&format!("\"{pid}\""))
}

fn note(key: Option<u8>, frame_length: u64, lyric: &str) -> ScoreNote {
    ScoreNote {
        key,
        frame_length,
        lyric: lyric.into(),
    }
}

fn scored(notes: Vec<ScoreNote>) -> Score {
    let mut starts = Vec::new();
    let mut at = 0;
    for n in &notes {
        starts.push(at);
        at += n.frame_length;
    }
    Score {
        voice: Default::default(),
        sources: vec![None; notes.len()],
        notes,
        starts,
    }
}

fn donguri() -> Score {
    let mut notes = vec![note(None, 15, "")];
    for (key, lyric) in [
        (67, "ど"),
        (64, "ん"),
        (64, "ぐ"),
        (65, "り"),
        (64, "こ"),
        (62, "ろ"),
    ] {
        notes.push(note(Some(key), 45, lyric));
    }
    notes.push(note(Some(60), 90, "こ"));
    notes.push(note(None, 15, ""));
    scored(notes)
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_engine_sings_and_the_cache_skips_it_the_second_time() {
    let engine = start(PORT);
    let client = &engine.client;
    let pid = engine.started_pid().unwrap();
    let version = client.version().unwrap();
    assert!(!version.is_empty());
    let root = tempfile::tempdir().unwrap();
    let score = donguri();
    let mut calls = 0;
    let mut sing = |json: &str, rate| {
        calls += 1;
        client.sing(json, rate)
    };
    let first = cache::load_or_sing(root.path(), &score, RATE, &mut sing).unwrap();
    assert_eq!(score.total_frames(), 390);
    assert_eq!(first.len() as u64, score.total_frames() * 512 * 2);
    assert!(first.iter().any(|s| s.abs() > 0.01), "silent wav");
    let again = cache::load_or_sing(root.path(), &score, RATE, &mut sing).unwrap();
    assert_eq!(calls, 1);
    assert_eq!(again, first);

    let bad = client.sing(
        &scored(vec![note(None, 15, ""), note(Some(60), 45, "どん")]).json(),
        RATE,
    );
    let error = bad.unwrap_err();
    assert!(error.contains("400"), "{error}");
    eprintln!("engine {version}; multi-mora lyric: {error}");

    drop(engine);
    let client = Client::new(&format!("http://127.0.0.1:{PORT}"));
    assert!(client.version().is_err(), "engine still answers");
    assert!(!running(pid));
}

#[test]
fn voicevox_executable_needs_an_existing_file() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("run.exe");
    std::fs::write(&file, b"").unwrap();
    assert_eq!(executable(None), None);
    assert_eq!(executable(Some(&root.path().join("missing.exe"))), None);
    assert_eq!(executable(Some(root.path())), None);
    assert_eq!(executable(Some(&file)), Some(file.as_path()));
}

#[test]
fn voicevox_connect_without_engine_or_executable_asks_for_one() {
    let root = tempfile::tempdir().unwrap();
    for configured in [None, Some(root.path().join("missing.exe"))] {
        let result = connect(50122, configured.as_deref(), Duration::from_secs(1));
        assert_eq!(result.err(), Some(Failure::NoExecutable));
    }
}

#[cfg(windows)]
#[test]
fn voicevox_connect_reports_an_executable_that_exits_without_answering() {
    // cmd.exe は `--port` を無視し、stdin が無いのですぐ終わる。
    let shell = std::path::PathBuf::from(std::env::var("ComSpec").unwrap());
    let Err(Failure::Failed(error)) = connect(50122, Some(&shell), Duration::from_secs(30)) else {
        panic!("cmd.exe must not count as an engine");
    };
    assert!(error.contains("exited before answering"), "{error}");
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_engine_started_here_is_stopped_and_a_running_one_is_kept() {
    let started = Instant::now();
    let owner = start(LIFECYCLE_PORT);
    let pid = owner.started_pid().unwrap();
    eprintln!("engine answered in {:?}", started.elapsed());
    assert!(running(pid));

    let shared = connect(LIFECYCLE_PORT, Some(&configured()), START_TIMEOUT).unwrap();
    assert_eq!(shared.started_pid(), None);
    drop(shared);
    assert!(
        owner.client.version().is_ok(),
        "a shared engine was stopped"
    );
    assert!(running(pid));

    drop(owner);
    let client = Client::new(&format!("http://127.0.0.1:{LIFECYCLE_PORT}"));
    assert!(client.version().is_err(), "engine still answers");
    assert!(!running(pid));
}

/// 1音の譜面（試聴で鳴らす長さ）を歌わせる時間。1回目は model の読み込みを含む。
#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_engine_sings_one_note_for_an_audition() {
    let engine = start(ONE_NOTE_PORT);
    let pid = engine.started_pid().unwrap();
    for (key, frames) in [(60, 47), (62, 47), (64, 23), (60, 47)] {
        let score = scored(vec![
            note(None, 15, ""),
            note(Some(key), frames, "ら"),
            note(None, 15, ""),
        ]);
        let started = Instant::now();
        let wav = engine.client.sing(&score.json(), RATE).unwrap();
        eprintln!(
            "one note key {key}, {frames} frames: {:?}, {} bytes",
            started.elapsed(),
            wav.len()
        );
    }
    drop(engine);
    assert!(!running(pid));
}
