//! The curve placed in a pane draws there instead of floating, and edits as the floating one does.
use super::variation_tests::fixture::{fixture as variation_fixture, index_curve, Fixture};
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key, text};
use egui::{Key, Modifiers};
use std::ops::{Deref, DerefMut};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

/// The curve above a mixer in a dock window.
const DOCK: &str = r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "parameter_curve" }, { view = "mixer" }]
"#;
const BESIDE_THE_LIST: &str = r#"
[root]
row = [{ view = "event_list" }, { view = "parameter_curve" }]
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Parameter,
    Variation,
}
const KINDS: [Kind; 2] = [Kind::Parameter, Kind::Variation];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Floating,
    Dock,
    Root,
}

/// The app, with the plugin the variation curve needs kept alive.
enum Held {
    Parameter(App),
    Variation(Fixture),
}
impl Deref for Held {
    type Target = App;
    fn deref(&self) -> &App {
        match self {
            Held::Parameter(a) => a,
            Held::Variation(f) => f,
        }
    }
}
impl DerefMut for Held {
    fn deref_mut(&mut self) -> &mut App {
        match self {
            Held::Parameter(a) => a,
            Held::Variation(f) => f,
        }
    }
}

fn layout(place: Place) -> crate::store::layout::Spec {
    crate::store::layout::parse(match place {
        Place::Floating => "",
        Place::Dock => DOCK,
        Place::Root => BESIDE_THE_LIST,
    })
    .unwrap()
}

fn placed(kind: Kind, place: Place) -> Held {
    let mut a = match kind {
        Kind::Parameter => Held::Parameter(super::tests::app()),
        Kind::Variation => {
            let mut f = variation_fixture();
            cat_smf_editor::score::macro_curve::write(
                &mut f.loaded.as_mut().unwrap().music.tracks[0],
                &index_curve(),
            )
            .unwrap();
            f.parameter_editor.selected = Some((crate::view::tempo_curve::Target::Point, 0));
            Held::Variation(f)
        }
    };
    a.layout.spec = layout(place);
    a
}

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

/// The window receiving the keys typed into the curve.
fn host(place: Place) -> egui::ViewportId {
    if place == Place::Dock {
        dock()
    } else {
        ROOT
    }
}

fn minimap() -> egui::Id {
    egui::Id::new((crate::view::timeline_controls::Screen::Parameter, "minimap"))
}

/// Draws one frame and tells whether the curve's timeline was drawn anywhere in it.
fn drawn(a: &mut App, c: &egui::Context) -> bool {
    c.data_mut(|d| d.remove::<egui::Rect>(minimap()));
    frame(a, c, ROOT, vec![]);
    c.data(|d| d.get_temp::<egui::Rect>(minimap()).is_some())
}

/// The curve's floating window in the main window, maximized or not.
fn floating(c: &egui::Context) -> bool {
    ["parameter-curve", "parameter-curve-maximized"]
        .into_iter()
        .any(|id| c.memory(|m| m.area_rect(egui::Id::new(id)).is_some()))
}

fn visible(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
        .unwrap_or(true)
}

fn shows_text(c: &egui::Context, id: egui::ViewportId, text: &str) -> bool {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
    .iter()
    .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == text))
}

fn press(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key) {
    key(a, c, id, k, Modifiers::NONE);
}

#[test]
fn each_placed_curve_draws_in_its_pane_and_floats_no_window() {
    for kind in KINDS {
        for place in [Place::Dock, Place::Root] {
            let mut a = placed(kind, place);
            let c = context();
            frame(&mut a, &c, ROOT, vec![]);
            assert!(drawn(&mut a, &c), "{kind:?} {place:?}");
            assert!(!floating(&c), "{kind:?} {place:?}");
            assert_eq!(focus::focused(&a), ViewKind::ParameterCurve);
            assert_eq!(owner(&a, &c), Screen::Timeline);
            // Taken out of the layout, the curve floats again.
            a.layout.spec = layout(Place::Floating);
            assert!(drawn(&mut a, &c), "{kind:?} {place:?}");
            assert!(floating(&c), "{kind:?} {place:?}");
        }
    }
}

#[test]
fn the_closed_curve_leaves_its_pane_empty_and_takes_no_focus() {
    let mut a = placed(Kind::Parameter, Place::Dock);
    let track = a.track;
    a.parameter_editor.close();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!drawn(&mut a, &c));
    // The key reaches the mixer below, passing over the empty pane.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // A click on the event list hands the focus back before the view opens from there.
    focus::set(&mut a, ViewKind::EventList);
    a.select_track(track);
    open(&mut a);
    assert!(drawn(&mut a, &c));
    assert!(!floating(&c));
    assert_eq!(focus::focused(&a), ViewKind::ParameterCurve);
    // Esc closes the curve, and the keys go back to the event list.
    press(&mut a, &c, dock(), Key::Escape);
    assert!(!a.parameter_editor.open);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert!(!drawn(&mut a, &c));
}

#[test]
fn moving_to_another_pane_closes_the_curve() {
    let mut a = placed(Kind::Parameter, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::F6);
    assert!(!a.parameter_editor.open);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // The emptied pane is passed over.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
}

/// What the user can observe after each input of one sequence.
#[derive(Debug, PartialEq)]
struct Step {
    owner: Screen,
    open: bool,
    selected: Option<(crate::view::tempo_curve::Target, usize)>,
    input: bool,
    error: Option<String>,
    bytes: Vec<u8>,
}

fn observe(a: &App, c: &egui::Context) -> Step {
    Step {
        owner: owner(a, c),
        open: a.parameter_editor.open,
        selected: a.parameter_editor.selected,
        input: a.parameter_editor.input_open(),
        error: a.parameter_editor.error.clone(),
        bytes: cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
    }
}

#[derive(Clone, Copy)]
enum Input {
    Key(Key, Modifiers),
    Text(&'static str),
}

fn sequence(kind: Kind) -> Vec<Input> {
    let k = |key| Input::Key(key, Modifiers::NONE);
    let mut keys = vec![k(Key::L), k(Key::Enter)];
    match kind {
        // The tick field of the curve's own input row.
        Kind::Parameter => {
            keys.extend(std::iter::repeat_n(k(Key::Backspace), 3));
            keys.push(Input::Text("360"));
        }
        // The value of a point is chosen in the variation overlay.
        Kind::Variation => keys.push(k(Key::ArrowUp)),
    }
    keys.extend([
        k(Key::Enter),
        k(Key::J),
        k(Key::Enter),
        k(Key::ArrowUp),
        k(Key::Enter),
        Input::Key(Key::Z, Modifiers::CTRL),
        k(Key::H),
        k(Key::Delete),
        k(Key::Escape),
    ]);
    keys
}

fn run(kind: Kind, place: Place) -> Vec<Step> {
    let mut a = placed(kind, place);
    let c = context();
    let id = host(place);
    frame(&mut a, &c, ROOT, vec![]);
    let mut steps = vec![observe(&a, &c)];
    for input in sequence(kind) {
        match input {
            Input::Key(k, m) => key(&mut a, &c, id, k, m),
            Input::Text(s) => text(&mut a, &c, id, s),
        }
        frame(&mut a, &c, ROOT, vec![]);
        if a.parameter_editor.input_open() {
            // The input stays in the window holding the curve, which stays shown.
            assert!(visible(&c, id), "{kind:?} {place:?}");
        }
        steps.push(observe(&a, &c));
    }
    steps
}

#[test]
fn a_placed_curve_edits_as_the_floating_one() {
    for kind in KINDS {
        let floating = run(kind, Place::Floating);
        let n = floating.len();
        // The first Enter opens the point's input, the next applies it; Ctrl+Z takes an edit back.
        assert!(floating[2].input, "{kind:?}");
        let applied = n - 9;
        assert_eq!(floating[applied].owner, Screen::Timeline, "{kind:?}");
        assert_ne!(floating[0].bytes, floating[applied].bytes, "{kind:?}");
        assert_ne!(floating[n - 6].bytes, floating[n - 5].bytes, "{kind:?}");
        assert_eq!(floating[n - 6].bytes, floating[n - 4].bytes, "{kind:?}");
        assert_ne!(floating[n - 2].bytes, floating[n - 3].bytes, "{kind:?}");
        assert!(!floating[n - 1].open, "{kind:?}");
        for place in [Place::Dock, Place::Root] {
            let pane = run(kind, place);
            assert_eq!(pane.len(), n);
            for (i, (pane, floating)) in pane.iter().zip(&floating).enumerate() {
                assert_eq!(pane, floating, "{kind:?} {place:?} step {i}");
            }
        }
    }
}

#[test]
fn the_variation_chooser_opens_in_the_dock_window_and_applies() {
    let mut a = placed(Kind::Variation, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::Enter);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(owner(&a, &c), Screen::ParameterInput);
    assert!(visible(&c, dock()));
    assert!(shows_text(&c, dock(), "Apply  Enter"));
    assert!(!c.memory(|m| m
        .area_rect(egui::Id::new("macro-variation-point"))
        .is_some()));
    press(&mut a, &c, dock(), Key::ArrowUp);
    press(&mut a, &c, dock(), Key::ArrowUp);
    press(&mut a, &c, dock(), Key::Enter);
    assert!(!a.parameter_editor.input_open());
    assert_eq!(curve_points(&a)[0], 2.0);
    assert!(drawn(&mut a, &c));
}

fn curve_points(a: &App) -> Vec<f64> {
    current(a).unwrap().points.iter().map(|p| p.value).collect()
}

#[test]
fn opening_the_curve_takes_the_focus_from_the_other_timelines_of_its_dock() {
    let mut a = placed(Kind::Parameter, Place::Floating);
    a.layout.spec = crate::store::layout::parse(
        r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "piano_roll" }, { view = "tempo_curve" }, { view = "parameter_curve" }]
"#,
    )
    .unwrap();
    let track = a.track;
    a.parameter_editor.close();
    let c = context();
    focus::set(&mut a, ViewKind::PianoRoll);
    a.tempo_curve.open = true;
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    a.select_track(track);
    open(&mut a);
    assert!(drawn(&mut a, &c));
    assert_eq!(focus::focused(&a), ViewKind::ParameterCurve);
    assert!(!a.piano_roll.open && !a.tempo_curve.open);
    // Moving to another timeline of the dock closes the curve.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    assert!(!a.parameter_editor.open);
}

#[test]
fn a_hidden_dock_window_drops_the_curve_drag() {
    let mut a = placed(Kind::Parameter, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    a.parameter_editor.drag = true;
    a.parameter_editor.draft = current(&a).ok();
    a.edit_menu.open = true;
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!visible(&c, dock()));
    assert!(!a.parameter_editor.drag);
    assert!(a.parameter_editor.draft.is_none());
}
