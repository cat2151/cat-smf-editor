use crate::{
    app::App,
    play::playback::{self, Mode},
};
use eframe::egui::{self, Key, Modifiers};

fn empty() -> App {
    let mut app = App::empty();
    app.audition = crate::play::audition::Audition::disabled();
    app
}

fn key(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
    let _ = ctx.run(
        egui::RawInput {
            modifiers,
            events: vec![egui::Event::Key {
                key,
                modifiers,
                pressed: true,
                repeat: false,
                physical_key: None,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    );
    let _ = ctx.run(
        egui::RawInput {
            modifiers,
            events: vec![egui::Event::Key {
                key,
                modifiers,
                pressed: false,
                repeat: false,
                physical_key: None,
            }],
            ..Default::default()
        },
        // A release pass also runs work deferred behind the busy modal.
        |ctx| crate::view::show(app, ctx),
    );
}

#[test]
fn windows_control_modifier_creates_song_and_adds_track() {
    let mut app = empty();
    let ctx = egui::Context::default();
    // egui-winit also sets the platform-neutral `command` flag for Ctrl on Windows.
    let ctrl = Modifiers {
        command: true,
        ..Modifiers::CTRL
    };
    key(&mut app, &ctx, Key::N, ctrl);
    assert!(app.loaded.is_some(), "Windows Ctrl+N must create a song");
    key(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    key(&mut app, &ctx, Key::N, ctrl | Modifiers::SHIFT);
    assert_eq!(app.track, 1);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 2);
}

#[test]
fn keyboard_new_add_play_and_undo_keep_unsaved_song_in_memory() {
    let mut app = empty();
    app.config.navigation.last_mid = Some("X:/music/previous.mid".into());
    let ctx = egui::Context::default();
    key(&mut app, &ctx, Key::N, Modifiers::CTRL);
    assert_eq!(app.loaded.as_ref().unwrap().name, "untitled.mid");
    assert!(app.saved.is_untitled() && app.saved.dirty());
    assert_eq!(app.track, 0);
    assert_eq!(app.playback.last, Mode::Track);
    assert_eq!(playback::parts(&app, Mode::Track).len(), 1);
    assert!(app.recent.history.entries.is_empty());
    assert_eq!(
        app.config.navigation.last_mid.as_ref().unwrap().to_str(),
        Some("X:/music/previous.mid")
    );
    key(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    key(&mut app, &ctx, Key::N, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(app.track, 1);
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        (
            loaded.music.tracks.len(),
            loaded.track_titles.len(),
            loaded.patches.len()
        ),
        (2, 2, 2)
    );
    let parts = playback::parts(&app, Mode::Track);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, 1);
    assert_eq!(parts[0].2.notes[0].velocity, 100);
    assert_eq!(parts[0].2.notes[0].duration, 0.5);
    key(&mut app, &ctx, Key::Z, Modifiers::CTRL);
    assert_eq!(app.track, 0);
    assert_eq!(app.loaded.as_ref().unwrap().track_titles.len(), 1);
    assert!(app.saved.dirty());
}

#[test]
fn new_song_respects_departure_and_overlay_ownership() {
    let mut app = empty();
    app.new_smf_unchecked();
    app.edit_style = None;
    app.edit_start = Default::default();
    app.add_track().unwrap();
    let ctx = egui::Context::default();
    app.comma_menu.open = true;
    key(&mut app, &ctx, Key::N, Modifiers::CTRL);
    assert!(app.config.navigation.departure.is_none());
    app.comma_menu.open = false;
    key(&mut app, &ctx, Key::N, Modifiers::CTRL);
    assert!(matches!(
        app.config.navigation.departure,
        Some(crate::view::files::Action::New)
    ));
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 2);
    key(&mut app, &ctx, Key::Escape, Modifiers::NONE);
    assert!(app.config.navigation.departure.is_none());
    key(&mut app, &ctx, Key::N, Modifiers::CTRL);
    key(&mut app, &ctx, Key::N, Modifiers::NONE);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 1);
}

#[test]
fn first_save_roundtrips_and_registers_real_path() {
    let root = tempfile::tempdir().unwrap();
    let mut app = empty();
    let status = root.path().join("status.json");
    app.config = crate::store::config::Store::open(status.clone());
    app.new_smf_unchecked();
    app.add_track().unwrap();
    let path = root.path().join("created.mid");
    assert!(!path.exists());
    app.saved.save_as(&path).unwrap();
    app.sync_saved_path();
    assert!(!app.saved.dirty() && !app.saved.is_untitled());
    let mut reopened = empty();
    reopened.config = crate::store::config::Store::open(status);
    reopened.open_initial(None);
    assert_eq!(reopened.loaded.as_ref().unwrap().music.tracks.len(), 2);
    assert_eq!(reopened.track, 1);
    assert!(reopened.load_error.is_none());
    assert_eq!(app.config.navigation.last_mid, Some(path));
}

#[test]
fn plus_is_below_tracks_and_click_selects_and_plays_appended_track() {
    let mut app = empty();
    app.new_smf_unchecked();
    app.edit_style = None;
    app.edit_start = Default::default();
    let ctx = egui::Context::default();
    let render = |app: &mut App, events| {
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
    let text = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
                _ => None,
            })
            .unwrap()
    };
    let plus = text("+") + egui::vec2(3.0, 6.0);
    assert!(plus.y > text("1 · —").y);
    render(
        &mut app,
        vec![
            egui::Event::PointerMoved(plus),
            egui::Event::PointerButton {
                pos: plus,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    render(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: plus,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(crate::view::busy::pending(&app));
    assert_eq!(app.track, 0);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 1);
    render(&mut app, vec![]);
    assert_eq!(app.track, 1);
    assert_eq!(app.loaded.as_ref().unwrap().music.tracks.len(), 2);
    assert_eq!(app.playback.last, Mode::Track);
    let parts = playback::parts(&app, Mode::Track);
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].0, 1);
}
