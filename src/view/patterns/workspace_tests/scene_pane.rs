//! The Scene placed in a pane draws there instead of in a window of its own, with the same keys.
use super::*;
use crate::view::commands::Screen;
use crate::view::dock::{focus, window, ViewKind};

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

/// `id` showed a window of its own since the last call.
fn opened(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data_mut(|d| d.remove_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
        .is_some()
}

/// The transport button of `p` was drawn since the last call.
fn drawn(c: &egui::Context, p: Pane) -> bool {
    let id = egui::Id::new((p.title(), "transport"));
    c.data_mut(|d| {
        let seen = d.get_temp::<egui::Rect>(id).is_some();
        d.remove::<egui::Rect>(id);
        seen
    })
}

fn placed_scene(views: &str) -> (App, egui::Context) {
    let (mut a, c, .., scene) = verification_song();
    workspace::open_scene(&mut a, scene).unwrap();
    place(&mut a, views);
    frame(&mut a, &c, ROOT, vec![]);
    opened(&c, Pane::Scene.viewport());
    drawn(&c, Pane::Scene);
    (a, c)
}

#[test]
fn the_placed_scene_draws_in_its_pane_and_opens_no_window() {
    let (mut a, c) = placed_scene(r#"{ view = "scene" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(drawn(&c, Pane::Scene));
    assert!(!opened(&c, Pane::Scene.viewport()));
    // The Song keeps its own window.
    assert!(opened(&c, Pane::Song.viewport()));
}

#[test]
fn the_scene_pane_stays_empty_and_takes_no_focus_until_the_scene_is_asked_for() {
    let (mut a, c) = placed_scene(r#"{ view = "scene" }"#);
    a.patterns.workspace.scene_window = false;
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!drawn(&c, Pane::Scene));
    assert!(!opened(&c, Pane::Scene.viewport()));
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    // `S` in the Song asks for the Scene again.
    key(&mut a, &c, Pane::Song.viewport(), Key::S, Modifiers::NONE);
    assert!(a.patterns.workspace.scene_window);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(drawn(&c, Pane::Scene));
    assert!(!opened(&c, Pane::Scene.viewport()));
}

#[test]
fn taking_the_scene_out_of_the_layout_brings_its_window_back() {
    let (mut a, c) = placed_scene(r#"{ view = "scene" }"#);
    a.layout.spec = crate::store::layout::parse("").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c, Pane::Scene.viewport()));
    assert!(drawn(&c, Pane::Scene));
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", Pane::Scene.viewport()))))
        .unwrap_or(false));
}

/// Ids are random per run, so parts are compared by name and lanes by row.
#[derive(Debug, PartialEq)]
struct Outcome {
    /// Row, measure and pattern of every cell of the open Scene.
    cells: Vec<(usize, u64, String)>,
    /// Row, measure, placing and chosen part of the Scene cursor.
    cell: (usize, u64, bool, Option<String>),
}

fn outcome(a: &App) -> Outcome {
    let music = &a.loaded.as_ref().unwrap().music;
    let collection = scene::read(music).unwrap();
    let names = choices(a, Pane::Scene);
    let name = |part: PatternOrScene| {
        names
            .iter()
            .find(|(p, _)| *p == part)
            .map_or_else(|| format!("{part:?}"), |(_, n)| n.clone())
    };
    let open = collection
        .scenes
        .iter()
        .find(|s| Some(s.id) == a.patterns.workspace.scene)
        .unwrap();
    let mut cells: Vec<_> = open
        .cells
        .iter()
        .map(|c| {
            let row = open.lanes.iter().position(|l| *l == c.lane).unwrap();
            (row, c.measure, name(PatternOrScene::Pattern(c.pattern)))
        })
        .collect();
    cells.sort();
    let s = &a.patterns.workspace.scene_cell;
    Outcome {
        cells,
        cell: (s.row, s.measure, s.placing, s.choice.map(name)),
    }
}

/// A return to the event list and back, selection moves, placements and an undo.
const KEYS: [(Key, Modifiers); 11] = [
    (Key::Escape, Modifiers::NONE),
    (Key::K, Modifiers::NONE),
    (Key::ArrowDown, Modifiers::NONE),
    (Key::ArrowRight, Modifiers::NONE),
    (Key::Enter, Modifiers::NONE),
    (Key::ArrowUp, Modifiers::NONE),
    (Key::Enter, Modifiers::NONE),
    (Key::Z, Modifiers::CTRL),
    (Key::Escape, Modifiers::NONE),
    (Key::Escape, Modifiers::NONE),
    (Key::K, Modifiers::NONE),
];

fn run(placed: bool) -> Vec<Outcome> {
    let (mut a, c) = if placed {
        placed_scene(r#"{ view = "scene" }"#)
    } else {
        let (mut a, c, .., scene) = verification_song();
        workspace::open_scene(&mut a, scene).unwrap();
        (a, c)
    };
    let id = if placed {
        dock()
    } else {
        Pane::Scene.viewport()
    };
    KEYS.iter()
        .map(|&(k, m)| {
            key(&mut a, &c, id, k, m);
            outcome(&a)
        })
        .collect()
}

#[test]
fn the_same_keys_edit_the_scene_alike_in_its_pane_and_its_window() {
    let window = run(false);
    let pane = run(true);
    for (i, (w, p)) in window.iter().zip(&pane).enumerate() {
        assert_eq!(w, p, "after {:?}", KEYS[i]);
    }
    // The keys did move, place and undo.
    assert_ne!(window[3].cell, window[1].cell, "{window:?}");
    assert_ne!(window[6].cells, window[5].cells, "{window:?}");
    assert_eq!(window[7].cells, window[5].cells);
}

#[test]
fn leaving_the_placed_scene_hands_the_keys_back_and_a_key_in_its_window_returns() {
    let (mut a, c) = placed_scene(r#"{ view = "scene" }"#);
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Scene);
    a.patterns.workspace.scene_cell.placing = false;
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Main);
    a.patterns.workspace.return_to = Some(Pane::Scene);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Scene);
}

#[test]
fn a_dialog_opened_in_the_scene_pane_holds_its_dock_window() {
    let (mut a, c) = placed_scene(r#"{ view = "scene" }, { view = "mixer" }"#);
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    command(&mut a, &c, dock(), "scene.create");
    assert_eq!(
        a.patterns.workspace.dialog.as_ref().map(|d| d.view),
        Some(ViewKind::Scene)
    );
    frame(&mut a, &c, dock(), vec![]);
    let field = egui::Id::new(("arrangement-field", "arrangement-name"));
    assert!(c.data(|d| d.get_temp::<egui::Rect>(field)).is_some());
    assert!(!opened(&c, Pane::Scene.viewport()));
    // Neither a key nor a press reaches the mixer below.
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    let mixer = egui::pos2(450.0, 500.0);
    for pressed in [true, false] {
        frame(
            &mut a,
            &c,
            dock(),
            vec![
                egui::Event::PointerMoved(mixer),
                egui::Event::PointerButton {
                    pos: mixer,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    assert!(a.patterns.workspace.input_open());
    click(&mut a, &c, dock(), field);
    text(&mut a, &c, dock(), "B");
    key(&mut a, &c, dock(), Key::Tab, Modifiers::NONE);
    key(&mut a, &c, dock(), Key::A, Modifiers::CTRL);
    text(&mut a, &c, dock(), "3");
    key(&mut a, &c, dock(), Key::Enter, Modifiers::NONE);
    assert!(
        !a.patterns.workspace.input_open(),
        "{:?}",
        a.patterns.workspace.error
    );
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let made = collection.scenes.iter().find(|s| s.name == "B").unwrap();
    // The new scene opens in the pane, which keeps the focus.
    assert_eq!(a.patterns.workspace.scene, Some(made.id));
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(drawn(&c, Pane::Scene));
    assert!(!opened(&c, Pane::Scene.viewport()));
    command(&mut a, &c, dock(), "scene.create");
    assert!(a.patterns.workspace.input_open());
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
}

#[test]
fn f6_steps_between_the_song_and_scene_panes_of_one_dock_window() {
    let (mut a, c) = placed_scene(r#"{ view = "song" }, { view = "scene" }"#);
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Scene);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    key(&mut a, &c, dock(), Key::F6, Modifiers::SHIFT);
    assert_eq!(focus::focused(&a), ViewKind::Scene);
    // An empty Scene pane is passed over.
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    a.patterns.workspace.scene_window = false;
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    assert_eq!(a.pane_focus, Some(ViewKind::Song));
}
