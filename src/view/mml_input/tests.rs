use super::*;
use cat_smf_editor::{
    model::{Music, Track},
    smf_io,
};

fn fixture() -> App {
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    })
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
}

/// The key pass paints the busy modal; the next pass opens the input.
fn open_input(app: &mut App, ctx: &egui::Context) {
    frame(app, ctx, vec![key(egui::Key::I)]);
    frame(app, ctx, vec![]);
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn keyboard_open_type_preview_navigate_confirm_and_undo() {
    let mut app = fixture();
    app.track_view.cursor_tick = Some(720);
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    frame(
        &mut app,
        &ctx,
        vec![key(egui::Key::I), egui::Event::Text("i".into())],
    );
    frame(&mut app, &ctx, vec![]);
    assert!(app.mml_input.open);
    assert!(app.mml_input.buffer.is_empty());
    frame(&mut app, &ctx, vec![egui::Event::Text("C F G".into())]);
    assert_eq!(app.mml_input.buffer, "C F G");
    assert!(!app.audition.auditioned.is_empty());
    let row = app.cursor_row;
    frame(&mut app, &ctx, vec![key(egui::Key::ArrowLeft)]);
    assert_eq!(app.cursor_row, row);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert!(!app.mml_input.open);
    assert!(!app.playback.open);
    assert!(!app.mml_input.buffer.contains('\n'));
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        loaded.music.tracks[0]
            .events
            .iter()
            .filter(|e| cat_smf_editor::score::note_view::note_on(e))
            .map(|e| e.tick)
            .min(),
        Some(720)
    );
    assert!(app.undo.available());
    app.undo_music().unwrap();
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn failed_confirmation_stays_open() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert!(app.mml_input.open && app.mml_input.error.is_some());
    assert!(!app.undo.available());
}

fn note_ticks(app: &App) -> Vec<u64> {
    app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| e.tick)
        .collect()
}

#[test]
fn typing_previews_in_song_and_escape_leaves_one_undo_step() {
    let mut app = fixture();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    assert_eq!(note_ticks(&app).len(), 1);
    frame(&mut app, &ctx, vec![egui::Event::Text("eg".into())]);
    assert_eq!(note_ticks(&app).len(), 3);
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(!app.mml_input.open && !app.help_open);
    assert_eq!(note_ticks(&app).len(), 3);
    app.undo_music().unwrap();
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
}

#[test]
fn erasing_input_restores_opening_song() {
    let mut app = fixture();
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    assert_eq!(note_ticks(&app).len(), 1);
    frame(&mut app, &ctx, vec![key(egui::Key::Backspace)]);
    assert!(app.mml_input.buffer.is_empty());
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn reopening_starts_empty() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert!(!app.mml_input.open, "{:?}", app.mml_input.error);
    // egui treats a second press without a release as a repeat.
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::I,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    open_input(&mut app, &ctx);
    assert!(app.mml_input.open);
    assert!(app.mml_input.buffer.is_empty());
}

#[test]
fn other_editors_and_menus_keep_their_i_key() {
    let ctx = egui::Context::default();
    let mut app = fixture();
    for kind in 0..5 {
        app.help_open = kind == 0;
        app.comma_menu.open = kind == 1;
        app.edit_menu.open = kind == 2;
        app.layer_editor.open = kind == 3;
        app.playback.open = kind == 4;
        let _ = ctx.run(
            egui::RawInput {
                events: vec![key(egui::Key::I)],
                ..Default::default()
            },
            |ctx| shortcuts(&mut app, ctx),
        );
        assert!(!crate::view::busy::pending(&app));
    }
}

#[test]
fn confirm_plays_once_from_insert_start_to_insert_end() {
    let mut app = fixture();
    app.track_view.cursor_tick = Some(720);
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("l4 c d".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    assert!(!app.mml_input.open);
    let prepared = app
        .playback
        .pending
        .as_ref()
        .expect("confirm starts playback");
    // 480 ticks per quarter at 120 BPM: start at 720 ticks, two quarter notes long.
    assert!((prepared.active.start_seconds - 0.75).abs() < 1e-9);
    assert!((prepared.active.end - 1.0).abs() < 1e-9);
    assert_eq!(app.playback.repeat, None);
}
