use crate::{
    player::selection::bulk_tests::identical,
    store::config::{ProgramTarget as Target, Store},
    view::tests::app,
};
use eframe::egui;

/// Presses and releases `key`; without the release egui reports the next press as a repeat.
fn press(app: &mut crate::app::App, ctx: &egui::Context, key: egui::Key) {
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        );
    }
}

#[test]
fn comma_t_c_and_comma_t_a_are_saved_and_t_follows_them_without_asking() {
    let root = std::env::temp_dir().join(format!("cat-smf-comma-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let status = root.join("status.json");
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    let ctx = egui::Context::default();
    assert_eq!(app.config.program_target, Target::Auto);
    for key in [egui::Key::Comma, egui::Key::T, egui::Key::C] {
        press(&mut app, &ctx, key);
        // Menu letters must not reach the t shortcut while the menu is open.
        assert!(app.player_selection.error.is_none());
    }
    assert!(!app.comma_menu.open);
    assert_eq!(Store::open(status.clone()).program_target, Target::Current);
    // Track 0 has no notes, so the current-track path stops at the error overlay.
    press(&mut app, &ctx, egui::Key::T);
    assert!(app.player_selection.no_notes_error);
    press(&mut app, &ctx, egui::Key::Escape);

    for key in [egui::Key::Comma, egui::Key::T, egui::Key::A] {
        press(&mut app, &ctx, key);
    }
    assert_eq!(Store::open(status.clone()).program_target, Target::Auto);
    // Identical programs go straight to all tracks; the fixture has no player.
    press(&mut app, &ctx, egui::Key::T);
    assert!(!app.player_selection.no_notes_error);
    assert!(app
        .player_selection
        .error
        .as_deref()
        .unwrap()
        .contains("executable"));
    std::fs::remove_file(&status).unwrap();
    std::fs::remove_dir(&root).unwrap();
}

#[test]
fn escape_closes_comma_menu_without_changing_target() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    for key in [egui::Key::Comma, egui::Key::T, egui::Key::Escape] {
        press(&mut app, &ctx, key);
    }
    assert!(!app.comma_menu.open);
    assert_eq!(app.config.program_target, Target::Auto);
}

#[test]
fn curve_maximization_defaults_on_and_can_be_saved_from_the_open_curve() {
    let root = std::env::temp_dir().join(format!("cat-smf-maximized-{}", std::process::id()));
    let status = root.join("status.json");
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    assert!(app.config.tempo_curve_maximized);
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.tempo_curve.open);
    press(&mut app, &ctx, egui::Key::Comma);
    assert!(app.comma_menu.open);
    press(&mut app, &ctx, egui::Key::M);
    assert!(!app.config.tempo_curve_maximized);
    assert!(!Store::open(status.clone()).tempo_curve_maximized);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.tempo_curve.open);
    assert!(!app.comma_menu.open);
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::M);
    assert!(Store::open(status.clone()).tempo_curve_maximized);
    std::fs::write(&status, r#"{"session":null}"#).unwrap();
    assert!(Store::open(status).tempo_curve_maximized);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn piano_roll_maximization_is_independent_persistent_and_defaults_on() {
    let root = std::env::temp_dir().join(format!("cat-smf-piano-maximized-{}", std::process::id()));
    let status = root.join("status.json");
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    assert!(app.config.piano_roll_maximized);
    app.select_track(1);
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::R);
    assert!(app.piano_roll.open);
    press(&mut app, &ctx, egui::Key::Comma);
    assert!(app.comma_menu.open);
    press(&mut app, &ctx, egui::Key::R);
    assert!(!app.config.piano_roll_maximized);
    assert!(app.config.tempo_curve_maximized);
    let saved = Store::open(status.clone());
    assert!(!saved.piano_roll_maximized && saved.tempo_curve_maximized);
    press(&mut app, &ctx, egui::Key::M);
    let saved = Store::open(status.clone());
    assert!(!saved.piano_roll_maximized && !saved.tempo_curve_maximized);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(app.piano_roll.open);
    assert!(!app.comma_menu.open);
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::R);
    let saved = Store::open(status.clone());
    assert!(saved.piano_roll_maximized && !saved.tempo_curve_maximized);
    std::fs::write(&status, r#"{"session":null,"tempo_curve_maximized":false}"#).unwrap();
    let saved = Store::open(status);
    assert!(saved.piano_roll_maximized && !saved.tempo_curve_maximized);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn meta_text_toggles_are_independent_persistent_and_shared_by_both_overlays() {
    let root = std::env::temp_dir().join(format!("cat-smf-meta-text-{}", std::process::id()));
    let status = root.join("status.json");
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    assert!(app.config.meta_text_visible && !app.config.meta_text_icons);
    let ctx = egui::Context::default();
    for track in [0, 1] {
        app.select_track(track);
        press(&mut app, &ctx, egui::Key::R);
        press(&mut app, &ctx, egui::Key::Comma);
        let before = app.config.meta_text_icons;
        press(&mut app, &ctx, egui::Key::I);
        assert_eq!(app.config.meta_text_icons, !before);
        assert!(app.config.meta_text_visible);
        let saved = Store::open(status.clone());
        assert_eq!(saved.meta_text_icons, !before);
        press(&mut app, &ctx, egui::Key::A);
        assert!(!Store::open(status.clone()).meta_text_visible);
        press(&mut app, &ctx, egui::Key::A);
        assert!(Store::open(status.clone()).meta_text_visible);
        press(&mut app, &ctx, egui::Key::Escape);
        assert!(!app.comma_menu.open);
        assert!(app.piano_roll.open || app.tempo_curve.open);
        press(&mut app, &ctx, egui::Key::Escape);
    }
    std::fs::write(&status, r#"{"session":null}"#).unwrap();
    let saved = Store::open(status);
    assert!(saved.meta_text_visible && !saved.meta_text_icons);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn spectrum_openness_is_saved_and_comma_p_forgets_its_window_without_closing_it() {
    let root = std::env::temp_dir().join(format!("cat-smf-spectrum-open-{}", std::process::id()));
    let status = root.join("status.json");
    let mut app = app(identical());
    app.config = Store::open(status.clone());
    app.config
        .select_analysis_window([-5000.0, -5000.0, 700.0, 250.0]);
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::Comma);
    press(&mut app, &ctx, egui::Key::F);
    assert!(Store::open(status.clone()).analysis_open);
    press(&mut app, &ctx, egui::Key::P);
    assert!(app.analysis.is_open());
    assert_eq!(Store::open(status.clone()).analysis_window, None);
    press(&mut app, &ctx, egui::Key::F);
    assert!(!Store::open(status.clone()).analysis_open);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn f1_opens_help_and_escape_closes_without_changing_the_view() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    press(&mut app, &ctx, egui::Key::F1);
    assert!(app.help_open);
    assert!(app.note_mode);
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.help_open);
    app.piano_roll.open = true;
    press(&mut app, &ctx, egui::Key::F1);
    assert!(app.help_open);
    press(&mut app, &ctx, egui::Key::F1);
    assert!(!app.help_open);
    assert!(app.piano_roll.open);
}

#[test]
fn hamburger_opens_the_same_help_settings_as_comma() {
    let mut app = app(identical());
    let ctx = egui::Context::default();
    let render = |app: &mut crate::app::App, events| {
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
    };
    render(&mut app, vec![]);
    let output = render(&mut app, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == "☰" => {
                Some(t.pos + egui::vec2(3.0, 3.0))
            }
            _ => None,
        })
        .expect("hamburger");
    assert!(pos.x > 1400.0, "right aligned: {pos:?}");
    for pressed in [true, false] {
        render(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(app.comma_menu.open);
    let output = render(&mut app, vec![]);
    assert!(output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "help")));
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.comma_menu.open);
}
