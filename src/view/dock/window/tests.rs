//! Views placed in a dock window take its keys and leave the main window's alone.
use super::*;
use crate::view::commands::Screen;
use crate::view::patterns::workspace_tests::{context, frame, key, request_close};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{Key, Modifiers};
mod every_view;
mod piano;
mod settles;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: if status == 0xff { data[0] } else { 0 },
        data: if status == 0xff {
            data[1..].into()
        } else {
            data.into()
        },
    }
}

fn app() -> App {
    let track = |key: u8| Track {
        events: vec![
            event(0, 0x90, &[key, 100]),
            event(480, 0x80, &[key, 0]),
            event(960, 0x90, &[key + 2, 100]),
            event(1440, 0x80, &[key + 2, 0]),
            event(1920, 0xff, &[0x2f]),
        ],
    };
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![track(60), track(64)],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    let mut a = crate::view::tests::app(music);
    a.layout.spec = crate::store::layout::parse(
        r#"
[root]
view = "event_list"

[[window]]
title = "Dock"
rect = [0, 0, 900, 700]
column = [{ view = "piano_roll" }, { view = "mixer" }]
"#,
    )
    .unwrap();
    a
}

fn dock() -> egui::ViewportId {
    viewport(0)
}

fn shows(c: &egui::Context, id: egui::ViewportId, text: &str) -> bool {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
    .iter()
    .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == text))
}

fn visible(c: &egui::Context) -> Option<bool> {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", dock()))))
}

fn floating(c: &egui::Context, id: &str) -> bool {
    c.memory(|m| m.area_rect(egui::Id::new(id)).is_some())
}

fn press(pos: egui::Pos2) -> Vec<egui::Event> {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(pos), button(true), button(false)]
}

#[test]
fn the_dock_window_draws_its_panes_instead_of_floating_windows() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    let builder = c
        .data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", dock()))))
        .unwrap();
    assert_eq!(builder.title.as_deref(), Some("Dock"));
    assert_eq!(visible(&c), Some(true));
    assert!(shows(&c, dock(), "Master effects"));
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!floating(&c, "Mixer (m)"));
}

#[test]
fn keys_typed_into_the_dock_window_reach_its_panes_and_not_the_event_list() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    let row = a.cursor_row;
    // The first key gives the focus to the window's first pane.
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    assert!(a.piano_roll.open);
    assert_eq!(a.cursor_row, row);
    key(&mut a, &c, dock(), Key::ArrowRight, Modifiers::CTRL);
    assert_eq!(a.track, 1);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    // Changing the track moved the cursor to its first row.
    let row = a.cursor_row;
    assert!(!floating(&c, "Piano roll"));
    // F6 stays within the dock window.
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!a.piano_roll.open);
    key(&mut a, &c, dock(), Key::L, Modifiers::NONE);
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(
        a.audition.gain(0),
        crate::play::track_volume::db_to_gain(-3)
    );
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    key(&mut a, &c, dock(), Key::F6, Modifiers::SHIFT);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // The main window's keys go to the focused pane, too.
    key(&mut a, &c, ROOT, Key::J, Modifiers::NONE);
    assert_eq!(a.cursor_row, row);
    assert_eq!(
        a.audition.gain(0),
        crate::play::track_volume::db_to_gain(-6)
    );
    // The palette opened in the dock window lists the focused pane's commands.
    key(
        &mut a,
        &c,
        dock(),
        Key::P,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    assert_eq!(a.palette.origin(), Some((dock(), Screen::Mixer)));
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.palette.is_open());
    // Escape hands the keys back to the event list.
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    key(&mut a, &c, ROOT, Key::J, Modifiers::NONE);
    assert_ne!(a.cursor_row, row);
    let row = a.cursor_row;
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    assert_eq!(a.cursor_row, row);
}

#[test]
fn a_press_in_a_dock_pane_takes_the_focus() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, dock(), vec![]);
    frame(&mut a, &c, dock(), press(egui::pos2(450.0, 600.0)));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    frame(&mut a, &c, dock(), press(egui::pos2(450.0, 150.0)));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    frame(&mut a, &c, ROOT, press(egui::pos2(700.0, 500.0)));
    assert_eq!(focus::focused(&a), ViewKind::EventList);
}

#[test]
fn a_menu_hides_the_dock_window_until_it_closes() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    a.edit_menu.open = true;
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(visible(&c), Some(false));
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(visible(&c), Some(true));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
}

#[test]
fn removing_the_window_from_the_layout_brings_its_views_back_floating() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert!(!floating(&c, "Mixer (m)"));
    a.layout.spec = crate::store::layout::parse("[root]\nview = \"event_list\"").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(floating(&c, "Mixer (m)"));
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Mixer);
}

#[test]
fn without_native_windows_the_dock_is_a_window_inside_the_main_one() {
    let mut a = app();
    let c = egui::Context::default();
    let at = crate::view::piano_roll::tests::at;
    at(&mut a, &c, 0.0, vec![]);
    let output = at(&mut a, &c, 1.0, vec![]);
    let inside = c.memory(|m| m.area_rect(egui::Id::new(("dock-window", 0))));
    assert!(inside.is_some());
    assert!(output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Master effects")));
    let keys = |pressed| egui::Event::Key {
        key: Key::M,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    at(&mut a, &c, 2.0, vec![keys(true), keys(false)]);
    at(&mut a, &c, 2.1, vec![]);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!floating(&c, "Mixer (m)"));
}

#[test]
fn the_close_button_quits_and_leaves_the_dock_window_open() {
    let mut a = app();
    let c = context();
    let commands = |c: &egui::Context| {
        c.data(|d| {
            d.get_temp::<Vec<egui::ViewportCommand>>(egui::Id::new(("workspace-commands", dock())))
        })
        .unwrap()
    };
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!commands(&c).contains(&egui::ViewportCommand::CancelClose));
    request_close(&c, dock());
    frame(&mut a, &c, ROOT, vec![]);
    assert!(commands(&c).contains(&egui::ViewportCommand::CancelClose));
    // The unsaved song asks before quitting.
    assert!(matches!(
        a.config.navigation.departure,
        Some(crate::view::files::Action::Close)
    ));
    // Declining keeps the window.
    a.config.navigation.departure = None;
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(visible(&c), Some(true));
    assert!(shows(&c, dock(), "Master effects"));
}
