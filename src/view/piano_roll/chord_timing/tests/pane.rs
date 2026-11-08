//! Chord timing opened from a placed roll shows in the window holding the roll.
use super::{song, spans};
use crate::app::App;
use crate::view::commands::{owner, Screen};
use crate::view::dock::window;
use crate::view::patterns::workspace_tests::{context, frame, key};
use eframe::egui::{self, Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

const DOCK: &str = r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
view = "piano_roll"
"#;
const BESIDE_THE_LIST: &str = r#"
[root]
row = [{ view = "event_list" }, { view = "piano_roll" }]
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Floating,
    Dock,
    Root,
}
const PLACES: [Place; 3] = [Place::Floating, Place::Dock, Place::Root];

/// The window holding the roll, and so the chord timing buttons.
fn host(place: Place) -> egui::ViewportId {
    if place == Place::Dock {
        window::viewport(0)
    } else {
        ROOT
    }
}

/// The roll opened with `R` in the main window, then chord timing with `C` in the roll's window.
fn opened(place: Place) -> (App, egui::Context) {
    let mut a = song();
    a.layout.spec = crate::store::layout::parse(match place {
        Place::Floating => "",
        Place::Dock => DOCK,
        Place::Root => BESIDE_THE_LIST,
    })
    .unwrap();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::R, Modifiers::NONE);
    key(&mut a, &c, host(place), Key::C, Modifiers::NONE);
    assert_eq!(owner(&a, &c), Screen::ChordTiming, "{place:?}");
    frame(&mut a, &c, ROOT, vec![]);
    (a, c)
}

fn visible(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
        .unwrap_or(true)
}

/// Centre of the button labelled `label` in the last frame of `id`.
fn button(c: &egui::Context, id: egui::ViewportId, label: &str) -> Option<egui::Pos2> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
    .iter()
    .find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == label => {
            Some(t.galley.rect.translate(t.pos.to_vec2()).center())
        }
        _ => None,
    })
}

fn click_at(a: &mut App, c: &egui::Context, id: egui::ViewportId, pos: egui::Pos2) {
    frame(a, c, id, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            a,
            c,
            id,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    frame(a, c, ROOT, vec![]);
}

fn click(a: &mut App, c: &egui::Context, id: egui::ViewportId, label: &str) {
    frame(a, c, ROOT, vec![]);
    let pos = button(c, id, label).unwrap_or_else(|| panic!("{label} is drawn"));
    click_at(a, c, id, pos);
}

#[test]
fn chord_timing_shows_in_the_window_holding_the_roll() {
    for place in PLACES {
        let (a, c) = opened(place);
        assert!(visible(&c, host(place)), "{place:?}");
        assert!(button(&c, host(place), "j: Next").is_some(), "{place:?}");
        if place == Place::Dock {
            assert!(button(&c, ROOT, "j: Next").is_none());
        }
        drop(a);
    }
}

/// What the user can observe after each input.
#[derive(Debug, PartialEq)]
struct Step {
    owner: Screen,
    active: bool,
    spans: Vec<(u64, u64, u8)>,
    repeat: Option<(u64, u64)>,
    error: bool,
}

fn observe(a: &App, c: &egui::Context) -> Step {
    Step {
        owner: owner(a, c),
        active: super::super::active(a),
        spans: spans(a),
        repeat: a.playback.repeat,
        error: a.piano_roll.error.is_some(),
    }
}

/// Buttons and keys of the mode, ending with its OK button.
fn run(place: Place) -> Vec<Step> {
    let (mut a, c) = opened(place);
    let id = host(place);
    let mut steps = vec![observe(&a, &c)];
    click(&mut a, &c, id, "l: Later 1/16");
    steps.push(observe(&a, &c));
    key(&mut a, &c, id, Key::L, Modifiers::NONE);
    steps.push(observe(&a, &c));
    click(&mut a, &c, id, "L: Longer 1/16");
    steps.push(observe(&a, &c));
    key(&mut a, &c, id, Key::J, Modifiers::NONE);
    steps.push(observe(&a, &c));
    click(&mut a, &c, id, "h: Earlier 1/16");
    steps.push(observe(&a, &c));
    click(&mut a, &c, id, "Enter: OK");
    steps.push(observe(&a, &c));
    steps
}

#[test]
fn the_buttons_and_keys_move_the_chord_as_in_the_floating_roll() {
    let floating = run(Place::Floating);
    // Each step changed what it should, so equal results are not both "nothing happened".
    assert_ne!(floating[0].spans, floating[1].spans, "button");
    assert_ne!(floating[1].spans, floating[2].spans, "key");
    assert_ne!(floating[2].spans, floating[3].spans, "Shift button");
    assert_ne!(floating[3].repeat, floating[4].repeat, "next chord");
    assert_ne!(floating[4].spans, floating[5].spans, "earlier");
    let last = floating.last().unwrap();
    assert!(!last.active && last.owner == Screen::Timeline);
    for place in [Place::Dock, Place::Root] {
        let pane = run(place);
        assert_eq!(pane.len(), floating.len());
        for (i, (p, f)) in pane.iter().zip(&floating).enumerate() {
            assert_eq!(p, f, "{place:?} step {i}");
        }
    }
}

#[test]
fn a_menu_holds_the_buttons_and_closing_it_gives_them_back() {
    for place in PLACES {
        let (mut a, c) = opened(place);
        let id = host(place);
        let later = button(&c, id, "l: Later 1/16").unwrap();
        let before = spans(&a);
        a.edit_menu.open = true;
        frame(&mut a, &c, ROOT, vec![]);
        if place == Place::Dock {
            assert!(!visible(&c, id));
        }
        click_at(&mut a, &c, id, later);
        assert_eq!(spans(&a), before, "{place:?}");
        key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
        assert!(!a.edit_menu.open, "{place:?}");
        frame(&mut a, &c, ROOT, vec![]);
        assert!(visible(&c, id), "{place:?}");
        assert_eq!(owner(&a, &c), Screen::ChordTiming, "{place:?}");
        click(&mut a, &c, id, "l: Later 1/16");
        assert_ne!(spans(&a), before, "{place:?}");
    }
}

/// The dock window starts its floating windows anew each frame, as the main window does.
#[test]
fn the_dock_window_registers_the_buttons_once_a_frame() {
    let (mut a, c) = opened(Place::Dock);
    for _ in 0..20 {
        frame(&mut a, &c, ROOT, vec![]);
    }
    assert_eq!(
        crate::view::presentation::tests::registered(&c, host(Place::Dock)),
        1
    );
}
