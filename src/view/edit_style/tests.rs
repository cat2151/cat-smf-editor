use super::*;

fn bytes(app: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
}

fn new_song() -> App {
    let mut app = App::empty();
    app.audition = crate::play::audition::Audition::disabled();
    assert!(app.install_new_song());
    app
}

fn frame(app: &mut App, ctx: &egui::Context, keys: &[Key]) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 900.0),
            )),
            events: keys
                .iter()
                .flat_map(|&key| {
                    [true, false].map(|pressed| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                })
                .collect(),
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

#[test]
fn event_list_is_drawn_under_the_dialog_and_on_its_closing_frame() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    let first = frame(&mut app, &ctx, &[]);
    assert!(first
        .shapes
        .iter()
        .any(|s| { matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "MBT") }));
    assert!(!first.shapes.iter().any(|s| {
        matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "editスタイルを選んでください")
    }));
    frame(&mut app, &ctx, &[]);
    let output = frame(&mut app, &ctx, &[]);
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect();
    let tracks = texts
        .iter()
        .position(|t| *t == "Tracks")
        .expect("track panel behind dialog");
    let events = texts
        .iter()
        .position(|t| *t == "MBT")
        .expect("event list behind dialog");
    let dialog = texts
        .iter()
        .position(|t| *t == "editスタイルを選んでください")
        .expect("style dialog");
    assert!(tracks < dialog && events < dialog);
    let output = frame(&mut app, &ctx, &[Key::Escape]);
    assert!(output
        .shapes
        .iter()
        .any(|s| { matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "MBT") }));
}

#[test]
fn new_song_pattern_starts_with_instrument_choice_and_cancel_preserves_music() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    assert!(app.edit_style.is_none());
    app.note_mode = false;
    let before = bytes(&app);
    frame(&mut app, &ctx, &[]);
    frame(&mut app, &ctx, &[Key::P]);
    assert!(app.edit_style.is_none());
    assert!(app.patterns.confirming_creation());
    assert!(!app.mml_input.open);
    frame(&mut app, &ctx, &[Key::Escape]);
    frame(&mut app, &ctx, &[]);
    assert!(!app.patterns.confirming_creation());
    assert!(app.edit_style.is_none());
    assert_eq!(bytes(&app), before);
    assert!(!app.undo.available());
}

#[test]
fn pattern_choice_does_not_create_a_placeholder_native_window() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| {
        choose(&mut app, ctx, Style::Pattern)
    });
    assert!(app.patterns.confirming_creation());
    assert!(!output
        .viewport_output
        .contains_key(&super::super::patterns::workspace::Pane::Song.viewport()));
}

fn enter_pattern(app: &mut App, ctx: &egui::Context, palette: bool) {
    frame(app, ctx, &[]);
    if palette {
        frame(app, ctx, &[Key::Escape]);
        super::super::palette::tests::open(app, ctx);
        super::super::palette::tests::query(app, ctx, "pattern");
        frame(app, ctx, &[Key::Enter]);
    } else {
        // Fixtures with an existing container have undo history, so idle does
        // not offer the untouched-new-song dialog automatically.
        if app.edit_style.is_none() {
            open(app);
        }
        frame(app, ctx, &[Key::Enter]);
    }
}

#[test]
fn both_pattern_entries_offer_instruments_without_mutating_a_new_song() {
    for palette in [false, true] {
        let mut app = new_song();
        let ctx = egui::Context::default();
        let before = bytes(&app);
        enter_pattern(&mut app, &ctx, palette);
        assert!(app.patterns.confirming_creation());
        frame(&mut app, &ctx, &[]);
        frame(&mut app, &ctx, &[]);
        let output = frame(&mut app, &ctx, &[]);
        for label in ["楽器を選んでください", "Surge XT / Init Saw", "Drums"] {
            assert!(
                output.shapes.iter().any(|s| {
                    matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label)
                }),
                "missing {label}, palette={palette}"
            );
        }
        assert_eq!(bytes(&app), before);
        assert!(!app.undo.available());
        frame(&mut app, &ctx, &[Key::Escape]);
        assert!(!app.patterns.confirming_creation());
        assert_eq!(bytes(&app), before);
    }
}

#[test]
fn both_pattern_entries_confirm_a_saw_container_into_mml_and_reopen_existing_work() {
    for palette in [false, true] {
        let mut app = new_song();
        // Supply an existing sound container so the test needs no installed plugin
        // or external drum catalog. The library still has no pattern bodies.
        app.edit_tracks(|m| {
            cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ())
        })
        .unwrap();
        let ctx = egui::Context::default();
        enter_pattern(&mut app, &ctx, palette);
        assert!(app.patterns.confirming_creation());
        frame(&mut app, &ctx, &[Key::Enter]);
        for _ in 0..3 {
            frame(&mut app, &ctx, &[]);
        }
        assert!(app.mml_input.open, "{:?}", app.load_error);
        assert!(app.mml_input.buffer.is_empty());
        assert_eq!(super::super::commands::owner(&app, &ctx), Screen::Mml);
        let edit = app.patterns.editing.as_ref().unwrap();
        assert!(edit.standalone);
        assert_eq!(Some(edit.pattern), app.patterns.selected);
        frame(&mut app, &ctx, &[Key::Escape]);
        assert!(!app.mml_input.open);
        let before = bytes(&app);
        // Once a body exists both entries should open Song, without creating another.
        open(&mut app);
        enter_pattern(&mut app, &ctx, palette);
        assert!(app.patterns.workspace_open);
        assert!(!app.patterns.confirming_creation() && !app.mml_input.open);
        assert_eq!(bytes(&app), before);
    }
}

#[test]
fn keyboard_selection_opens_direct_without_leaking_enter_or_text() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, &[]);
    frame(&mut app, &ctx, &[Key::ArrowDown]);
    assert_eq!(app.edit_style, Some(Style::Direct));
    frame(&mut app, &ctx, &[Key::Enter, Key::I]);
    frame(&mut app, &ctx, &[]);
    assert!(app.edit_style.is_none() && app.mml_input.open);
    assert!(app.mml_input.buffer.is_empty());
    assert!(!app.patterns.workspace_open);
    frame(&mut app, &ctx, &[Key::Escape]);
    assert!(!app.mml_input.open);
}

#[test]
fn dialog_blocks_background_shortcuts_and_escape_does_not_reopen_it() {
    let mut app = new_song();
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, &[]);
    frame(&mut app, &ctx, &[Key::I, Key::V, Key::G]);
    assert!(app.edit_style.is_some());
    assert!(!app.mml_input.open && !app.track_view.go_open);
    frame(&mut app, &ctx, &[Key::Escape]);
    frame(&mut app, &ctx, &[]);
    assert!(app.edit_style.is_none());
}

#[test]
fn opening_an_existing_document_clears_the_new_song_prompt() {
    let mut app = new_song();
    let file = tempfile::Builder::new().suffix(".mid").tempfile().unwrap();
    std::fs::write(
        file.path(),
        cat_smf_editor::smf_io::encode(&cat_smf_editor::edit::tracks::new_song()).unwrap(),
    )
    .unwrap();
    app.load_mid_unchecked(file.path().to_owned());
    assert!(app.load_error.is_none());
    assert!(app.edit_style.is_none());
}

#[test]
fn both_style_choices_work_with_the_mouse() {
    for label in ["P: Patternスタイル", "D: Directスタイル"] {
        let mut app = new_song();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, &[]);
        frame(&mut app, &ctx, &[]);
        let output = frame(&mut app, &ctx, &[]);
        let position = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
                _ => None,
            })
            .expect("style choice is visible")
            + egui::vec2(4.0, 4.0);
        for pressed in [true, false] {
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ctx| crate::view::show(&mut app, ctx),
            );
        }
        frame(&mut app, &ctx, &[]);
        assert!(app.edit_style.is_none());
        if label.starts_with('P') {
            assert!(app.patterns.confirming_creation());
        } else {
            assert!(app.mml_input.open);
        }
    }
}
