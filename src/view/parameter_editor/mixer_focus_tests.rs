//! A curve opened while the mixer holds the keys, through the inputs the user has.
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, ViewKind};
use crate::view::patterns::workspace_tests::{command, context, frame, key};
use egui::{Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Floating,
    Dock,
    Root,
}
const PLACES: [Place; 3] = [Place::Floating, Place::Dock, Place::Root];

fn app(place: Place) -> App {
    let mut a = super::tests::app();
    a.layout.spec = crate::store::layout::parse(match place {
        Place::Floating => "",
        Place::Dock => {
            r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "parameter_curve" }, { view = "mixer" }]
"#
        }
        Place::Root => {
            r#"
[root]
row = [{ view = "event_list" }, { view = "parameter_curve" }, { view = "mixer" }]
"#
        }
    })
    .unwrap();
    a
}

/// The mixer holds the keys after `M`, with the curve of the current lane closed.
fn mixer_focused(place: Place) -> (App, egui::Context) {
    let mut a = app(place);
    a.parameter_editor.close();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert!(a.playback.mixer_focused(), "{place:?}");
    assert_eq!(owner(&a, &c), Screen::Mixer, "{place:?}");
    if place != Place::Floating {
        assert_eq!(focus::focused(&a), ViewKind::Mixer, "{place:?}");
    }
    (a, c)
}

/// Where the keys go after one input meant to open the curve.
#[derive(Debug, PartialEq)]
struct After {
    open: bool,
    owner: Screen,
    focused: ViewKind,
}

fn after(a: &App, c: &egui::Context) -> After {
    After {
        open: a.parameter_editor.open,
        owner: owner(a, c),
        focused: focus::focused(a),
    }
}

/// The placed curve takes the keys wherever the floating one does.
fn assert_as_floating(results: Vec<(Place, After)>, what: &str) {
    let floating = &results[0].1;
    assert_eq!(results[0].0, Place::Floating);
    for (place, after) in &results[1..] {
        assert_eq!(after.open, floating.open, "{what} {place:?}");
        assert_eq!(after.owner, floating.owner, "{what} {place:?}");
        let expected = if after.open {
            ViewKind::ParameterCurve
        } else {
            ViewKind::Mixer
        };
        assert_eq!(after.focused, expected, "{what} {place:?}");
    }
}

#[test]
fn the_keys_that_open_the_curve_from_the_event_list_reach_the_mixer_instead() {
    for (k, screen) in [(Key::T, Screen::Mixer), (Key::X, Screen::Effects)] {
        let results = PLACES
            .map(|place| {
                let (mut a, c) = mixer_focused(place);
                key(&mut a, &c, ROOT, k, Modifiers::NONE);
                frame(&mut a, &c, ROOT, vec![]);
                (place, after(&a, &c))
            })
            .into_iter()
            .collect::<Vec<_>>();
        assert!(!results[0].1.open, "{k:?}");
        assert_eq!(results[0].1.owner, screen, "{k:?}");
        assert_as_floating(results, &format!("{k:?}"));
    }
}

#[test]
fn the_palette_hands_the_keys_from_the_mixer_to_the_curve_it_opens() {
    for id in ["track.patch", "layer.open", "effect.open"] {
        let results = PLACES
            .map(|place| {
                let (mut a, c) = mixer_focused(place);
                command(&mut a, &c, ROOT, id);
                frame(&mut a, &c, ROOT, vec![]);
                (place, after(&a, &c))
            })
            .into_iter()
            .collect::<Vec<_>>();
        assert!(results[0].1.open, "{id}");
        assert_eq!(results[0].1.owner, Screen::Timeline, "{id}");
        assert_as_floating(results, id);
    }
}

/// The lane's header button in the main window, which opens its curve.
fn header(c: &egui::Context) -> Option<egui::Pos2> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", ROOT)))
    })
    .unwrap_or_default()
    .iter()
    .find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text().ends_with("/ Filter / Cutoff") => {
            Some(t.galley.rect.translate(t.pos.to_vec2()).left_center())
        }
        _ => None,
    })
}

fn click(a: &mut App, c: &egui::Context, pos: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            a,
            c,
            ROOT,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}

/// The floating mixer opens over the header; its title bar drags it out of the way.
fn drag_the_mixer_down(a: &mut App, c: &egui::Context) {
    let rect = c
        .memory(|m| m.area_rect(egui::Id::new("Mixer (m)")))
        .expect("floating mixer");
    let from = egui::pos2(rect.center().x, rect.top() + 8.0);
    let to = from + egui::vec2(0.0, 500.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(a, c, ROOT, vec![egui::Event::PointerMoved(from)]);
    frame(a, c, ROOT, vec![button(from, true)]);
    for step in 1..=5 {
        let pos = from + (to - from) * (step as f32 / 5.0);
        frame(a, c, ROOT, vec![egui::Event::PointerMoved(pos)]);
    }
    frame(a, c, ROOT, vec![button(to, false)]);
    frame(a, c, ROOT, vec![]);
}

#[test]
fn a_click_on_the_lane_header_hands_the_keys_from_the_mixer_to_the_curve() {
    let results = PLACES
        .map(|place| {
            let (mut a, c) = mixer_focused(place);
            if place == Place::Floating {
                drag_the_mixer_down(&mut a, &c);
            }
            let pos = header(&c).expect("header shown");
            assert_eq!(
                c.layer_id_at(pos).map(|l| l.order),
                Some(egui::Order::Background),
                "{place:?}"
            );
            click(&mut a, &c, pos);
            frame(&mut a, &c, ROOT, vec![]);
            (place, after(&a, &c))
        })
        .into_iter()
        .collect::<Vec<_>>();
    assert!(results[0].1.open);
    assert_eq!(results[0].1.owner, Screen::Timeline);
    assert_as_floating(results, "click");
}
