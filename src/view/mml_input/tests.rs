use super::*;
mod measure;
mod pattern;
mod positioning;
mod repeat;
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

fn keyed_fixture(sharps: i8, minor: bool) -> App {
    let mut music = fixture().loaded.unwrap().music;
    music.tracks[0].events.push(cat_smf_editor::model::Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![sharps as u8, u8::from(minor)],
    });
    crate::view::tests::app(music)
}

#[test]
fn chord_key_prefix_preserves_typing_cursor_preview_confirm_and_cancel() {
    let mut app = keyed_fixture(4, false);
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("I".into())]);
    assert_eq!(app.mml_input.buffer, "Key:E I");
    let pitches = |app: &App| {
        app.loaded.as_ref().unwrap().music.tracks[0]
            .events
            .iter()
            .filter(|e| cat_smf_editor::score::note_view::note_on(e))
            .map(|e| e.data[0] % 12)
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(pitches(&app), [4, 8, 11].into());
    frame(&mut app, &ctx, vec![egui::Event::Text(" V".into())]);
    assert_eq!(app.mml_input.buffer, "Key:E I V");
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    open(&mut app, &ctx, false);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("I".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![]);
    assert!(!app.mml_input.open);
    assert_eq!(pitches(&app), [4, 8, 11].into());
}

#[test]
fn key_prefix_uses_minor_tonic_and_preserves_explicit_keys_and_mml() {
    for (input, expected) in [
        ("I", "Key:C# Aeolian I"),
        ("Key:Bb I", "Key:Bb I"),
        ("o5 l8 cde", "o5 l8 cde"),
    ] {
        let mut app = keyed_fixture(4, true);
        let ctx = egui::Context::default();
        open_input(&mut app, &ctx);
        frame(&mut app, &ctx, vec![egui::Event::Text(input.into())]);
        assert_eq!(app.mml_input.buffer, expected);
        if input == "I" {
            assert!(cmrt_chord::resolve_chord_or_mml(expected).from_chord);
            assert_eq!(note_ticks(&app).len(), 3);
        }
    }
    let mut app = keyed_fixture(1, true); // E minor
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("I".into())]);
    assert_eq!(app.mml_input.buffer, "Key:E Aeolian I");
}

#[test]
fn prefix_follows_insertion_tick_and_format_two_track() {
    let mut music = keyed_fixture(4, false).loaded.unwrap().music;
    let mut later = music.tracks[0].events[0].clone();
    later.tick = 480;
    later.data = vec![255, 0]; // F major
    music.tracks[0].events.push(later);
    let mut other = music.tracks[0].clone();
    other.events.truncate(1);
    other.events[0].data = vec![2, 0]; // D major
    music.tracks.push(other);
    music.format = 2;
    for (track, tick, expected) in [(0, 0, "Key:E I"), (0, 480, "Key:F I"), (1, 480, "Key:D I")] {
        let mut app = crate::view::tests::app(music.clone());
        app.track = track;
        app.track_view.cursor_tick = Some(tick);
        let ctx = egui::Context::default();
        open_input(&mut app, &ctx);
        frame(&mut app, &ctx, vec![egui::Event::Text("I".into())]);
        assert_eq!(app.mml_input.buffer, expected);
    }
}

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
    raw_frame(app, ctx, events);
    if app.mml_input.pending_preview.is_some() {
        raw_frame(app, ctx, vec![]);
    }
}

fn raw_frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
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
    frame(&mut app, &ctx, vec![]);
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
    frame(&mut app, &ctx, vec![]);
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
fn first_input_paints_waiting_below_text_before_song_update_and_audition() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    let output = raw_frame(&mut app, &ctx, vec![egui::Event::Text("c".into())]);
    assert!(app.mml_input.pending_preview.is_some());
    assert!(note_ticks(&app).is_empty());
    assert!(app.audition.auditioned.is_empty());
    let waiting = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text)
                if text.galley.text() == "Preparing playback, please wait..." =>
            {
                Some(text.pos)
            }
            _ => None,
        })
        .expect("waiting message must be in the presented input frame");
    let input = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "c" => Some(text.pos),
            _ => None,
        })
        .unwrap();
    assert!(waiting.y > input.y && waiting.y - input.y < 150.0);
    let output = raw_frame(&mut app, &ctx, vec![]);
    assert_eq!(note_ticks(&app).len(), 1);
    assert_eq!(app.audition.auditioned.len(), 1);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::epaint::Shape::Text(text) if text.galley.text() == "Preparing playback, please wait...")));
}

#[test]
fn typing_previews_in_song_and_escape_restores_without_undo() {
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
    assert_eq!(note_ticks(&app).len(), 0);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
}

#[test]
fn input_layout_retry_keeps_preparation_until_the_next_presented_frame() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    let mut passes = 0;
    let output = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Text("c".into())],
            ..Default::default()
        },
        |ctx| {
            crate::view::show(&mut app, ctx);
            passes += 1;
            if passes == 1 {
                ctx.request_discard("exercise input layout retry");
            }
        },
    );
    assert!(passes > 1);
    assert!(note_ticks(&app).is_empty());
    assert!(app.audition.auditioned.is_empty());
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::epaint::Shape::Text(text) if text.galley.text() == "Preparing playback, please wait...")));
    raw_frame(&mut app, &ctx, vec![]);
    assert_eq!(note_ticks(&app).len(), 1);
    assert_eq!(app.audition.auditioned.len(), 1);
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
    frame(&mut app, &ctx, vec![]);
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
    for kind in 1..5 {
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
    frame(&mut app, &ctx, vec![]);
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

#[test]
fn whole_note_confirm_advances_to_next_measure_and_next_insertion() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c1".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(crate::play::playback::location(&app).0, 1920);
    assert_eq!(crate::play::playback::location(&app).1.unwrap().measure, 2);
    open(&mut app, &ctx, false);
    assert_eq!(app.mml_input.tick, 1920);
}

#[test]
fn cancel_preserves_previous_undo_and_exact_cursor() {
    let mut app = fixture();
    let previous = app.loaded.as_ref().unwrap().music.clone();
    app.undo.push(previous.clone());
    app.track_view.cursor_tick = Some(720);
    let ctx = egui::Context::default();
    open_input(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Text("c1".into())]);
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert_eq!(crate::play::playback::location(&app).0, 720);
    assert!(app.undo.available());
    app.undo_music().unwrap();
    assert!(!app.undo.available());
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        smf_io::encode(&previous).unwrap()
    );
}
