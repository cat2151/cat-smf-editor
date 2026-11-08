//! The keyboard placed in a pane draws and records there instead of in a window of its own.
use super::*;
use crate::view::dock::{focus, window, ViewKind};
use cat_smf_editor::score::pattern::recording::Pad;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

fn place(a: &mut App, views: &str) {
    a.layout.spec = crate::store::layout::parse(&format!(
        r#"
[[window]]
title = "Dock"
rect = [0, 0, 900, 600]
column = [{views}]
"#
    ))
    .unwrap();
}

fn keyboard() -> egui::ViewportId {
    crate::view::recording::window::viewport()
}

/// `id` showed a window of its own since the last call.
fn opened(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data_mut(|d| d.remove_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
        .is_some()
}

/// A pad was drawn since the last call.
fn pads(c: &egui::Context) -> bool {
    let id = egui::Id::new(("pad-rect", Pad::Row { row: 0, column: 0 }));
    c.data_mut(|d| {
        let seen = d.get_temp::<egui::Rect>(id).is_some();
        d.remove::<egui::Rect>(id);
        seen
    })
}

/// Holds `k` down without releasing it.
fn hold(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key) {
    let event = egui::Event::Key {
        key: k,
        physical_key: Some(k),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(a, c, id, vec![event]);
}

#[test]
fn the_placed_keyboard_draws_in_its_pane_and_opens_no_window() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    pads(&c);
    frame(&mut a, &c, ROOT, vec![]);
    // Without a session the pane is empty and cannot hold the focus.
    assert!(!pads(&c));
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    // Opening hands the keys to the pane, as a new window takes the focus.
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-focus", dock()))))
        .unwrap_or(false));
    frame(&mut a, &c, ROOT, vec![]);
    assert!(pads(&c));
    assert!(!opened(&c, keyboard()));
    assert_eq!(
        crate::view::commands::owner(&a, &c),
        crate::view::commands::Screen::Recording
    );
}

/// Records Z and C after Shift+R in the Song window; pads typed there do not play.
fn take(placed: bool) -> (Vec<u8>, usize, bool) {
    let (mut a, c, ..) = verification_song();
    let target = if placed {
        place(&mut a, r#"{ view = "keyboard" }"#);
        dock()
    } else {
        keyboard()
    };
    let before = normal(&a).len();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    pad(&mut a, &c, Pane::Song.viewport(), Key::Z);
    pad(&mut a, &c, target, Key::Space);
    pad(&mut a, &c, target, Key::Z);
    pad(&mut a, &c, target, Key::C);
    key(&mut a, &c, target, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
    let id = a.patterns.selected.unwrap();
    let revealed =
        a.patterns.workspace.selection(Pane::Song).choice == Some(PatternOrScene::Pattern(id));
    (body(&a, id), normal(&a).len() - before, revealed)
}

#[test]
fn the_same_keys_record_the_same_take_in_the_pane_and_in_the_window() {
    let window = take(false);
    assert_eq!(window, (vec![60, 64], 1, true));
    assert_eq!(take(true), window);
}

#[test]
fn keys_of_another_pane_of_the_dock_window_do_not_play_the_pads() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "song" }, { view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    key(&mut a, &c, dock(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    hold(&mut a, &c, dock(), Key::Z);
    assert!(crate::view::recording::sounding(&a, 60));
    pad(&mut a, &c, dock(), Key::Z);
    // F6 moves between the panes as in the other views.
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    hold(&mut a, &c, dock(), Key::Z);
    assert!(!crate::view::recording::sounding(&a, 60));
}

#[test]
fn a_take_from_the_song_pane_is_chosen_in_the_song_pane() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "song" }, { view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    key(&mut a, &c, dock(), Key::R, Modifiers::SHIFT);
    let library = a.patterns.editing.clone();
    for k in [Key::Space, Key::Z, Key::C] {
        pad(&mut a, &c, dock(), k);
    }
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    let id = a.patterns.selected.unwrap();
    assert_eq!(body(&a, id), vec![60, 64]);
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(id))
    );
    // The take is not opened in the editor, as from the Song window.
    assert_eq!(a.patterns.editing, library);
    // The Song keeps the keyboard, which records anew in its pane.
    assert!(a.patterns.recording.is_open());
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
}

#[test]
fn closing_the_placed_keyboard_empties_its_pane_and_returns_the_keys() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let c = context();
    place(&mut a, r#"{ view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(pads(&c));
    // Without patterns nothing keeps the keyboard, so its Esc closes it as its close button did.
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.recording.is_open());
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert_eq!(a.pane_focus, Some(ViewKind::EventList));
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!pads(&c));
    assert!(!opened(&c, keyboard()));
}

#[test]
fn taking_the_keyboard_out_of_the_layout_brings_its_window_back() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!opened(&c, keyboard()));
    a.layout.spec = crate::store::layout::parse("").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    pads(&c);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c, keyboard()));
    assert!(pads(&c));
    pad(&mut a, &c, keyboard(), Key::Z);
    assert!(a.patterns.recording.is_open());
}

#[test]
fn the_keyboard_placed_in_the_main_window_plays_its_keys() {
    let (mut a, c, ..) = verification_song();
    a.layout.spec = crate::store::layout::parse(
        r#"
[root]
row = [{ view = "event_list" }, { view = "keyboard" }]
"#,
    )
    .unwrap();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(pads(&c));
    assert!(!opened(&c, keyboard()));
    hold(&mut a, &c, ROOT, Key::Z);
    assert!(crate::view::recording::sounding(&a, 60));
}
