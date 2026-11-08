use super::*;
use crate::view::commands::{self, owner};
use crate::view::patterns::workspace_tests::{context, frame, key};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{Key, Modifiers};

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

/// Note mode, so the built-in layout puts the score beside the event list.
fn app() -> App {
    let mut a = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                event(0, 0x90, &[60, 100]),
                event(480, 0x80, &[60, 0]),
                event(960, 0x90, &[62, 100]),
                event(1440, 0x80, &[62, 0]),
                event(1920, 0xff, &[0x2f]),
            ],
        }],
    });
    a.note_mode = true;
    a
}

fn has(a: &App, screen: Screen, id: &str) -> bool {
    commands::definitions(a, screen).iter().any(|c| c.id == id)
}

#[test]
fn moving_the_focus_changes_the_owner() {
    let mut a = app();
    let c = egui::Context::default();
    assert_eq!(owner(&a, &c), Screen::Main);
    step(&mut a, true);
    assert_eq!(owner(&a, &c), Screen::Score);
    step(&mut a, true);
    assert_eq!(owner(&a, &c), Screen::Main);
}

#[test]
fn a_menu_owns_the_keys_whichever_pane_has_the_focus() {
    let mut a = app();
    let c = egui::Context::default();
    a.pane_focus = Some(ViewKind::Score);
    a.edit_menu.open = true;
    assert_eq!(owner(&a, &c), Screen::Edit);
    a.edit_menu.open = false;
    crate::view::comma_menu::open(&mut a);
    assert_eq!(owner(&a, &c), Screen::Settings);
}

#[test]
fn a_focus_outside_the_layout_falls_back_to_the_event_list() {
    let mut a = app();
    a.pane_focus = Some(ViewKind::Score);
    a.note_mode = false;
    assert_eq!(focused(&a), ViewKind::EventList);
    assert!(!has(&a, Screen::Main, "pane.focus-next"));
}

#[test]
fn keys_alone_go_round_the_panes() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    let row = a.cursor_row;
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(owner(&a, &c), Screen::Score);
    // The event list does not see the keys while the score has the focus.
    key(&mut a, &c, ROOT, Key::J, Modifiers::NONE);
    assert_eq!(a.cursor_row, row);
    // App-wide keys still work there.
    key(&mut a, &c, ROOT, Key::Comma, Modifiers::CTRL);
    assert_eq!(owner(&a, &c), Screen::Settings);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert_eq!(owner(&a, &c), Screen::Score);
    key(&mut a, &c, ROOT, Key::O, Modifiers::CTRL);
    assert!(a.file_open_requested);
    a.file_open_requested = false;
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(owner(&a, &c), Screen::Main);
    key(&mut a, &c, ROOT, Key::F6, Modifiers::SHIFT);
    assert_eq!(owner(&a, &c), Screen::Score);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert_eq!(owner(&a, &c), Screen::Main);
    key(&mut a, &c, ROOT, Key::J, Modifiers::NONE);
    assert_ne!(a.cursor_row, row);
}

#[test]
fn a_press_gives_the_pane_the_focus() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    let press = |x: f32| {
        let pos = egui::pos2(x, 500.0);
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ]
    };
    frame(&mut a, &c, ROOT, press(1400.0));
    assert_eq!(owner(&a, &c), Screen::Score);
    frame(&mut a, &c, ROOT, press(400.0));
    assert_eq!(owner(&a, &c), Screen::Main);
}
