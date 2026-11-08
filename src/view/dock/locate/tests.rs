use super::*;
use crate::view::patterns::workspace_tests::{command, frame, key, text, verification_song};
use eframe::egui::{Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

const LAYOUT: &str = r#"
[root]
row = [{ view = "event_list" }, { view = "score" }]

[[window]]
title = "Dock"
rect = [0, 0, 900, 700]
column = [{ view = "piano_roll" }, { view = "mixer" }]
"#;

fn app() -> App {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.layout.spec = crate::store::layout::parse(LAYOUT).unwrap();
    a
}

/// One frame of window `id` alone.
fn input(id: egui::ViewportId, events: Vec<egui::Event>) -> egui::RawInput {
    let mut input = egui::RawInput {
        viewport_id: id,
        events,
        ..Default::default()
    };
    input.viewports.insert(id, Default::default());
    input
}

/// What `view_at` says inside a frame of `id`.
fn at(a: &App, id: egui::ViewportId) -> Option<ViewKind> {
    let c = egui::Context::default();
    let mut view = None;
    let _ = c.run(input(id, vec![]), |ctx| view = view_at(a, ctx));
    view
}

#[test]
fn panes_answer_with_their_window_s_focused_pane() {
    let mut a = app();
    let dock = window::viewport(0);
    assert_eq!(at(&a, ROOT), Some(ViewKind::EventList));
    assert_eq!(at(&a, dock), Some(ViewKind::PianoRoll));
    focus::set(&mut a, ViewKind::Score);
    assert_eq!(at(&a, ROOT), Some(ViewKind::Score));
    assert_eq!(at(&a, dock), Some(ViewKind::PianoRoll));
    focus::set(&mut a, ViewKind::Mixer);
    // The main window keeps the event list while the focus is in a dock window.
    assert_eq!(at(&a, ROOT), Some(ViewKind::EventList));
    assert_eq!(at(&a, dock), Some(ViewKind::Mixer));
    for (view, id) in [(ViewKind::EventList, ROOT), (ViewKind::Mixer, dock)] {
        assert_eq!(viewport_of(&a, view), id);
    }
}

#[test]
fn windows_of_their_own_answer_with_their_view() {
    use crate::view::recording;
    let a = app();
    let own = [
        (crate::view::help::viewport(), ViewKind::Help),
        (a.analysis.viewport(), ViewKind::Analysis),
        (Pane::Song.viewport(), ViewKind::Song),
        (Pane::Scene.viewport(), ViewKind::Scene),
        (recording::window::viewport(), ViewKind::Keyboard),
    ];
    for (id, view) in own {
        assert_eq!(at(&a, id), Some(view), "{id:?}");
        assert_eq!(viewport_of(&a, view), id, "{view:?}");
    }
    // The keyboard's companion windows belong to the keyboard.
    for id in [
        recording::settings_viewport(),
        recording::piano_viewport(),
        egui::ViewportId::from_hash_of("virtual-midi-keyboard-capture"),
    ] {
        assert_eq!(at(&a, id), Some(ViewKind::Keyboard), "{id:?}");
    }
    assert_eq!(at(&a, egui::ViewportId::from_hash_of("elsewhere")), None);
}

fn suspended(a: &App, id: egui::ViewportId) -> bool {
    let c = egui::Context::default();
    let mut suspended = false;
    let _ = c.run(input(id, vec![egui::Event::Text("x".into())]), |ctx| {
        crate::view::patterns::workspace::suspend_background(a, ctx);
        suspended = ctx.input(|i| i.events.is_empty());
    });
    suspended
}

/// The dialog's input belongs to the view that opened it, even beside other panes.
#[test]
fn a_dialog_stops_the_other_view_of_its_window() {
    let (mut a, c, _, _, _) = verification_song();
    a.layout.spec = crate::store::layout::parse(LAYOUT).unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    command(&mut a, &c, ROOT, "scene.structure");
    key(&mut a, &c, ROOT, Key::A, Modifiers::CTRL);
    text(&mut a, &c, ROOT, "4/4");
    key(&mut a, &c, ROOT, Key::Enter, Modifiers::NONE);
    assert!(a.patterns.workspace.input_open());
    assert_eq!(
        a.patterns.workspace.dialog.as_ref().map(|d| d.view),
        Some(ViewKind::EventList)
    );
    assert!(!suspended(&a, ROOT));
    assert!(suspended(&a, Pane::Song.viewport()));
    // Another view of the same window holds the focus.
    focus::set(&mut a, ViewKind::Score);
    assert!(suspended(&a, ROOT));
    focus::set(&mut a, ViewKind::EventList);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
}

/// A key pressed in a dock window while the main window's dialog is open leaves the dialog its keys.
#[test]
fn a_dock_window_does_not_take_a_dialog_s_keys() {
    let (mut a, c, _, _, _) = verification_song();
    a.layout.spec = crate::store::layout::parse(LAYOUT).unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    focus::set(&mut a, ViewKind::Score);
    command(&mut a, &c, ROOT, "scene.structure");
    key(&mut a, &c, ROOT, Key::A, Modifiers::CTRL);
    text(&mut a, &c, ROOT, "4/4");
    key(&mut a, &c, ROOT, Key::Enter, Modifiers::NONE);
    assert!(a.patterns.workspace.input_open());
    let opener = a.patterns.workspace.dialog.as_ref().map(|d| d.view);
    key(&mut a, &c, window::viewport(0), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), opener.unwrap());
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
}
