//! MML 入力中の試聴を実 engine で通す。repo root の `.voicevox-engine-path`（git 管理外）に
//! `run.exe` のパスを1行で書き、`cargo test -- --ignored voicevox` で走らせる。
use super::flow_tests::{key, new_song, palette, SINGER};
use crate::app::App;
use crate::play::audio_sequence::{Note, Sequence};
use crate::play::audition::offline::without_plugins;
use crate::view::palette::tests::frame;
use cat_smf_editor::voicevox::{
    cache,
    client::Client,
    pcm,
    score::{one_note, LEAD_FRAMES},
};
use eframe::egui::{self, Key, Modifiers};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// 他の engine test と重ならない port。
const PORT: u16 = 50130;

fn configured() -> std::path::PathBuf {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/.voicevox-engine-path");
    std::fs::read_to_string(file)
        .expect(".voicevox-engine-path")
        .trim()
        .into()
}

fn wait(app: &mut App, ctx: &egui::Context) -> Duration {
    let started = Instant::now();
    while crate::play::sung::prerender::running(app) {
        assert!(started.elapsed() < Duration::from_secs(90));
        frame(app, ctx, vec![]);
        std::thread::sleep(Duration::from_millis(10));
    }
    started.elapsed()
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_the_second_mml_audition_sounds_the_engine_s_one_note() {
    let client = Client::new(&format!("http://127.0.0.1:{PORT}"));
    assert!(client.version().is_err(), "port {PORT} is already in use");
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(PORT, root.path(), None);
    app.config.voicevox_engine = Some(configured());
    palette(&mut app, &ctx, SINGER);
    println!("engine start and the song: {:?}", wait(&mut app, &ctx));
    assert_eq!(app.voicevox.error, None);
    let pid = app
        .voicevox
        .engine
        .as_ref()
        .and_then(|engine| engine.started_pid())
        .expect("started by the editor");

    // Measure 2 holds no note, so nothing loops over the audition.
    crate::cursor::moves::at_tick(&mut app, 1920);
    key(&mut app, &ctx, Key::I, Modifiers::NONE);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    frame(&mut app, &ctx, vec![]);
    assert!(app.audition.sung.is_none());
    println!("one note: {:?}", wait(&mut app, &ctx));
    assert_eq!(app.voicevox.error, None);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    frame(&mut app, &ctx, vec![]);
    let take = app.audition.sung.clone().expect("the voice");
    let pitch = app.audition.auditioned[0];
    crate::play::sung::prerender::shut_down(&mut app);
    let tasks = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&tasks.stdout).contains(&format!("\"{pid}\"")));

    // MML's default length is an eighth: 0.25 s at 120 BPM = 23 score frames.
    let rate = app.audition.rate();
    let sung: Vec<u64> = (1..200)
        .filter(|&frames| {
            let key = cache::key(&one_note(pitch, frames), rate);
            root.path().join(format!("{key}.wav")).exists()
        })
        .collect();
    // 47 is the song: `Ctrl+N`'s quarter C4 is the same score as one note of 47 frames.
    assert_eq!(sung, [23, 47]);
    let file = root
        .path()
        .join(format!("{}.wav", cache::key(&one_note(pitch, 23), rate)));
    let (_, wav) = pcm::decode(&std::fs::read(file).unwrap()).unwrap();
    let head = LEAD_FRAMES as usize * 512 * 2;
    let length = (wav.len() - head) / 2;
    let seconds = length as f64 / f64::from(rate);
    let note = Sequence::new(
        vec![Note {
            start: 0.0,
            duration: seconds,
            pitch,
            velocity: 100,
        }],
        seconds,
    );
    let out = without_plugins(rate, vec![(note, Some(take))], vec![1.0])
        .run(&AtomicBool::new(false))
        .unwrap()
        .unwrap();
    let body = out.len() - 2 * rate as usize / 100;
    let loud = out[..body].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let worst = out[..body]
        .iter()
        .zip(&wav[head..])
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    println!("key {pitch}: {length} frames, peak {loud}, worst |diff| {worst}");
    assert!(loud > 0.05, "the audition is silent ({loud})");
    assert_eq!(worst, 0.0);
}
