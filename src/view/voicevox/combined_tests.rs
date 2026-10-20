use super::measure_lyrics_tests::{
    assert_looping, lyrics, on_note, owned, singing, tracks, MEASURE,
};
use crate::view::{
    edit_menu::lyric_tests::{settle, wav},
    palette::tests::{frame, open, press, query},
};
use eframe::egui::{self, Key};
fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
    Ok(wav(rate))
}

#[test]
fn both_orders_preview_confirm_next_measure_and_one_undo() {
    for text in ["o5 l2 c d どん", "どん o5 l2 c d"] {
        let root = tempfile::tempdir().unwrap();
        let mut app = singing(root.path(), sing);
        let ctx = egui::Context::default();
        on_note(&mut app, 2400);
        frame(&mut app, &ctx, vec![]);
        let before = tracks(&app);
        open(&mut app, &ctx);
        query(&mut app, &ctx, "同時に差し替え");
        press(&mut app, &ctx, Key::Enter);
        frame(&mut app, &ctx, vec![]);
        assert!(app.voicevox.lyrics.as_ref().unwrap().combined);
        assert_looping(&app);
        frame(&mut app, &ctx, vec![egui::Event::Text(text.into())]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(
            lyrics(&app),
            owned(&[(480, "あ"), (1920, "ど"), (2880, "ん"), (3840, "お")])
        );
        assert_looping(&app);
        press(&mut app, &ctx, Key::Enter);
        frame(&mut app, &ctx, vec![]);
        assert!(app.voicevox.lyrics.is_none());
        assert_eq!(app.track_view.cursor_tick, Some(MEASURE.1));
        assert_looping(&app);
        settle(&mut app, &ctx);
        app.undo_music().unwrap();
        assert_eq!(tracks(&app), before);
    }
}

#[test]
fn escape_restores_both_notes_and_lyrics() {
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    on_note(&mut app, 2400);
    frame(&mut app, &ctx, vec![]);
    let before = tracks(&app);
    open(&mut app, &ctx);
    query(&mut app, &ctx, "同時に差し替え");
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("o5 c1 どん".into())]);
    press(&mut app, &ctx, Key::Escape);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert_eq!(tracks(&app), before);
}

#[test]
fn empty_measure_starts_loop_after_input_and_oversized_input_stays_open() {
    for (text, valid) in [("o5 c1 どん", true), ("o5 c1d1 どん", false)] {
        let root = tempfile::tempdir().unwrap();
        let mut app = singing(root.path(), sing);
        let ctx = egui::Context::default();
        crate::cursor::moves::at_tick(&mut app, 5760);
        frame(&mut app, &ctx, vec![]);
        let before = tracks(&app);
        open(&mut app, &ctx);
        query(&mut app, &ctx, "同時に差し替え");
        press(&mut app, &ctx, Key::Enter);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![egui::Event::Text(text.into())]);
        frame(&mut app, &ctx, vec![]);
        press(&mut app, &ctx, Key::Enter);
        frame(&mut app, &ctx, vec![]);
        if valid {
            assert!(app.voicevox.lyrics.is_none());
            assert_eq!(app.playback.repeat, Some((5760, 7680)));
            assert_eq!(app.track_view.cursor_tick, Some(7680));
        } else {
            assert!(app.voicevox.lyrics.is_some());
            assert_eq!(tracks(&app), before);
        }
    }
}

#[test]
fn next_empty_measure_keeps_the_previous_loop_until_combined_input_can_play() {
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    on_note(&mut app, 2400);
    frame(&mut app, &ctx, vec![]);
    open(&mut app, &ctx);
    query(&mut app, &ctx, "同時に差し替え");
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("o5 l2 c d どん".into())],
    );
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert_eq!(app.track_view.cursor_tick, Some(MEASURE.1));
    assert_looping(&app);
    settle(&mut app, &ctx);

    let empty = (5760, 7680);
    crate::cursor::moves::at_tick(&mut app, empty.0);
    open(&mut app, &ctx);
    query(&mut app, &ctx, "同時に差し替え");
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert_looping(&app);
    assert_eq!(app.voicevox.lyrics.as_ref().unwrap().replacing.range, empty);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("o5 l2 e f ぐり".into())],
    );
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.playback.repeat, Some(empty));
    assert!(crate::play::transport::timeline::running(&app));
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert_eq!(app.track_view.cursor_tick, Some(empty.1));
}
