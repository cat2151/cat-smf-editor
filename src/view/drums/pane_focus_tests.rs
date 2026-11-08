//! A placed grid stays open while another pane holds the keys.
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key};
use egui::Modifiers;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

/// The grid, the Virtual MIDI keyboard and the mixer in one dock window.
const DOCK: &str = r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "step_sequencer" }, { view = "keyboard" }, { view = "piano" }, { view = "mixer" }]
"#;

/// The grid opened in the dock window, holding the keys.
fn docked(fixture: fn() -> App) -> (App, egui::Context) {
    let mut a = fixture();
    a.layout.spec = crate::store::layout::parse(DOCK).unwrap();
    take_keys(&mut a);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    (a, c)
}

fn press(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key, m: Modifiers) {
    key(a, c, id, k, m);
    frame(a, c, ROOT, vec![]);
}

fn kit() -> App {
    let mut a = super::kit_edit_tests::fixture(4);
    open(&mut a);
    a
}

const FIXTURES: [fn() -> App; 3] = [super::tests::fixture, kit, super::melody_tests::fixture];

#[test]
fn the_grid_stays_open_while_the_keyboard_holds_the_keys() {
    for fixture in FIXTURES {
        let (mut a, c) = docked(fixture);
        press(&mut a, &c, window::viewport(0), Key::R, Modifiers::SHIFT);
        assert!(is_open(&a));
        assert_eq!(focus::focused(&a), ViewKind::Keyboard);
        assert_eq!(owner(&a, &c), Screen::Recording);
    }
}

#[test]
fn closing_the_keyboard_hands_the_keys_back_to_the_open_grid() {
    let (mut a, c) = docked(super::tests::fixture);
    let dock = window::viewport(0);
    press(&mut a, &c, dock, Key::R, Modifiers::SHIFT);
    press(&mut a, &c, dock, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.recording.is_open());
    assert!(is_open(&a));
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    assert_eq!(owner(&a, &c), Screen::Drums);
}

#[test]
fn undo_from_another_pane_leaves_the_keys_there() {
    let (mut a, c) = docked(super::melody_tests::fixture);
    let dock = window::viewport(0);
    let notes = |a: &App| {
        cat_smf_editor::score::step_notes::notes(
            &a.loaded.as_ref().unwrap().visible().tracks[a.track],
        )
        .len()
    };
    let before = notes(&a);
    press(&mut a, &c, dock, Key::F, Modifiers::NONE);
    assert_ne!(notes(&a), before);
    focus::set(&mut a, ViewKind::Mixer);
    press(&mut a, &c, dock, Key::Z, Modifiers::CTRL);
    assert_eq!(notes(&a), before);
    assert!(is_open(&a));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(owner(&a, &c), Screen::Mixer);
}

#[test]
fn the_event_list_takes_the_main_window_keys_beside_the_open_grid() {
    let (mut a, c) = docked(super::tests::fixture);
    focus::set(&mut a, ViewKind::EventList);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(is_open(&a));
    assert_eq!(owner(&a, &c), Screen::Main);
    // Opening the grid again gives it the keys.
    open(&mut a);
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    assert_eq!(owner(&a, &c), Screen::Drums);
}
