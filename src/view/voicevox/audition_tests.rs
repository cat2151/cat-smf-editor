//! MML 入力中の試聴: 歌う track では、1音の歌声が無ければ track の音色で鳴らして裏で歌わせ、
//! 次に同じ音を鳴らすときは歌声で鳴らす。
use super::flow_tests::{key, new_song, palette, SINGER};
use crate::app::App;
use crate::view::{edit_menu::lyric_tests::settle, palette::tests::frame};
use eframe::egui::{self, Key, Modifiers};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// `i` in measure 2, which holds no note, and one MML character, with the frame its audition
/// waits for.
fn type_mml(app: &mut App, ctx: &egui::Context, text: &str) {
    if !app.mml_input.open {
        crate::cursor::moves::at_tick(app, 1920);
        key(app, ctx, Key::I, Modifiers::NONE);
        assert!(app.mml_input.open);
    }
    frame(app, ctx, vec![egui::Event::Text(text.into())]);
    frame(app, ctx, vec![]);
}

/// The sung keys of an engine request.
fn keys(json: &str) -> Vec<u64> {
    let body: serde_json::Value = serde_json::from_str(json).unwrap();
    body["notes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["key"].as_u64())
        .collect()
}

#[test]
fn a_singer_sounds_the_instrument_first_then_the_voice_rendered_meanwhile() {
    static SUNG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.lock().unwrap().push(json.to_owned());
        Ok(crate::view::edit_menu::lyric_tests::wav(rate))
    }
    let sung = || SUNG.lock().unwrap().clone();
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(50126, root.path(), Some(sing));
    palette(&mut app, &ctx, SINGER);
    // The song itself (one note at tick 0) sings first; only one-note renders are counted below.
    settle(&mut app, &ctx);
    let before = sung().len();

    type_mml(&mut app, &ctx, "c");
    let pitch = app.audition.auditioned.clone();
    assert_eq!(pitch.len(), 1);
    assert!(app.audition.sung.is_none(), "the instrument sounds first");
    settle(&mut app, &ctx);
    let rendered = sung().split_off(before);
    assert_eq!(rendered.len(), 1, "{rendered:?}");
    assert_eq!(keys(&rendered[0]), [u64::from(pitch[0])]);
    assert!(rendered[0].contains("\"lyric\":\"ら\""));

    type_mml(&mut app, &ctx, "c");
    assert_eq!(app.audition.auditioned, pitch);
    assert!(app.audition.sung.is_some(), "the cached voice sounds");
    assert!(!crate::play::sung::prerender::running(&app));
    assert_eq!(sung().len(), before + 1);
}

#[test]
fn a_track_that_does_not_sing_renders_nothing() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(crate::view::edit_menu::lyric_tests::wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(50126, root.path(), Some(sing));
    type_mml(&mut app, &ctx, "c");
    type_mml(&mut app, &ctx, "c");
    assert_eq!(app.audition.auditioned.len(), 1);
    assert!(app.audition.sung.is_none());
    assert!(!crate::play::sung::prerender::running(&app));
    assert_eq!(SUNG.load(Ordering::SeqCst), 0);
}
