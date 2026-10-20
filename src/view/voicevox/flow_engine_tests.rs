//! 新規SMF からパレットだけで歌わせる流れを、実 engine で通す。repo root の
//! `.voicevox-engine-path`（git 管理外）に `run.exe` のパスを1行で書き、
//! `cargo test -- --ignored voicevox` で走らせる。
use super::flow_tests::{lyrics, new_song, palette, play, DONGURI, SINGER};
use crate::play::audition::offline::without_plugins;
use crate::play::playback::Mode;
use cat_smf_editor::voicevox::{cache, client::Client, pcm, score::from_track};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// 他の engine test と重ならない port。
const PORT: u16 = 50128;
/// 量子化 1 段ぶんの2倍まで。
const TOLERANCE: f32 = 2.0 / 32768.0;
/// `fade_out` が触る末尾は比べない。
const FADE: usize = 48000 / 100;

fn configured() -> std::path::PathBuf {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/.voicevox-engine-path");
    std::fs::read_to_string(file)
        .expect(".voicevox-engine-path")
        .trim()
        .into()
}

fn engines() -> String {
    let tasks = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq run.exe", "/FO", "CSV", "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&tasks.stdout).into_owned()
}

#[test]
#[ignore = "requires a VOICEVOX engine at .voicevox-engine-path"]
fn voicevox_a_new_song_sung_from_the_palette_renders_the_engine_wav() {
    let client = Client::new(&format!("http://127.0.0.1:{PORT}"));
    assert!(client.version().is_err(), "port {PORT} is already in use");
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(PORT, root.path(), None);
    app.config.voicevox_engine = Some(configured());

    palette(&mut app, &ctx, SINGER);
    lyrics(&mut app, &ctx, 0, "どんぐり");
    lyrics(&mut app, &ctx, 1920, "ころころ");
    let started = Instant::now();
    while crate::play::sung::prerender::running(&app) {
        assert!(started.elapsed() < Duration::from_secs(90));
        crate::view::palette::tests::frame(&mut app, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("engine start and singing: {:?}", started.elapsed());
    assert_eq!(app.voicevox.error, None);
    let pid = app
        .voicevox
        .engine
        .as_ref()
        .and_then(|engine| engine.started_pid())
        .expect("started by the editor");

    let track = play(&mut app, &ctx);
    assert_eq!(app.playback.last, Mode::Track);
    let music = &app.loaded.as_ref().unwrap().music;
    let score = from_track(music, track).unwrap();
    let lyrics: Vec<_> = score
        .notes
        .iter()
        .filter(|n| n.key.is_some())
        .map(|n| n.lyric.as_str())
        .collect();
    assert_eq!(lyrics, DONGURI);
    let rate = app.audition.rate();
    let file = root
        .path()
        .join(format!("{}.wav", cache::key(&score, rate)));
    let (wav_rate, wav) = pcm::decode(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(wav_rate, rate);

    // The take the playback was given, drawn offline on the track's own sequence.
    let take = app
        .audition
        .takes
        .iter()
        .find(|(t, _)| *t == track)
        .unwrap()
        .1
        .clone();
    let sequence = crate::play::transport::prepare(&app, Mode::Track)
        .unwrap()
        .parts
        .into_iter()
        .find(|(t, _, _)| *t == track)
        .unwrap()
        .2;
    let sequence_starts: Vec<f64> = sequence.notes.iter().map(|n| n.start).collect();
    let out = without_plugins(rate, vec![(sequence, Some(take))], vec![1.0])
        .run(&AtomicBool::new(false))
        .unwrap()
        .unwrap();

    crate::play::sung::prerender::shut_down(&mut app);
    let after = engines();
    assert!(
        !after.contains(&format!("\"{pid}\"")),
        "engine {pid} is still running: {after}"
    );

    let body = out.len() / 2 - FADE;
    let loud = out[..body * 2].iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(loud > 0.05, "the singer's track is silent ({loud})");
    // Each note-on plays its piece of the engine WAV from the head until the next note-on or the
    // piece's end, whichever comes first; at this tempo the pieces are rounded to whole frames, so
    // a short piece leaves silence before the next note-on and a long one is cut by it.
    let mut ons: Vec<usize> = sequence_starts
        .iter()
        .map(|s| (s * f64::from(rate)).round() as usize)
        .collect();
    ons.dedup();
    let sung: Vec<usize> = (0..score.notes.len())
        .filter(|&j| score.notes[j].key.is_some())
        .collect();
    assert_eq!(ons.len(), sung.len());
    let frame = |f: u64| f as usize * 512;
    let (mut worst, mut silent, mut cut) = (0.0f32, 0, 0);
    for (n, &j) in sung.iter().enumerate() {
        let piece = frame(score.starts[j])
            ..sung
                .get(n + 1)
                .map_or(frame(score.total_frames()), |&k| frame(score.starts[k]));
        let next = ons.get(n + 1).copied().unwrap_or(body).min(body);
        let played = piece.len().min(next - ons[n]);
        for k in 0..played * 2 {
            worst = worst.max((out[ons[n] * 2 + k] - wav[piece.start * 2 + k]).abs());
        }
        assert!(out[(ons[n] + played) * 2..next * 2]
            .iter()
            .all(|v| *v == 0.0));
        silent += next - ons[n] - played;
        cut += piece.len().saturating_sub(next - ons[n]);
    }
    println!("{body} frames, peak {loud}, worst |diff| {worst}, silent {silent}, cut {cut}");
    assert!(worst <= TOLERANCE);
}
