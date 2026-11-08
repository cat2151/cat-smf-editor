//! The Song placed in a pane draws there instead of in a window of its own, with the same keys.
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

/// The Song's transport button was drawn since the last call.
fn drawn(c: &egui::Context) -> bool {
    let id = egui::Id::new((Pane::Song.title(), "transport"));
    c.data_mut(|d| {
        let seen = d.get_temp::<egui::Rect>(id).is_some();
        d.remove::<egui::Rect>(id);
        seen
    })
}

fn shapes(c: &egui::Context, id: egui::ViewportId) -> usize {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .map_or(0, |s| s.len())
}

fn placed_song() -> (App, egui::Context) {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "song" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    opened(&c, Pane::Song.viewport());
    drawn(&c);
    (a, c)
}

#[test]
fn the_placed_song_draws_in_its_pane_and_opens_no_window() {
    let (mut a, c) = placed_song();
    frame(&mut a, &c, ROOT, vec![]);
    assert!(drawn(&c));
    assert!(shapes(&c, dock()) > 0);
    assert!(!opened(&c, Pane::Song.viewport()));
    // The Scene keeps its own window.
    assert!(opened(&c, Pane::Scene.viewport()));
}

#[test]
fn taking_the_song_out_of_the_layout_brings_its_window_back() {
    let (mut a, c) = placed_song();
    a.layout.spec = crate::store::layout::parse("").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c, Pane::Song.viewport()));
    assert!(drawn(&c));
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", Pane::Song.viewport()))))
        .unwrap_or(false));
}

/// Ids are random per run, so parts are compared by name and lanes by row.
#[derive(Debug, PartialEq)]
struct Outcome {
    /// Row, measure and part of every Song placement.
    song: Vec<(usize, u64, String)>,
    /// Row, measure, placing and chosen part of the Song cursor.
    cell: (usize, u64, bool, Option<String>),
}

fn outcome(a: &App) -> Outcome {
    let music = &a.loaded.as_ref().unwrap().music;
    let collection = scene::read(music).unwrap();
    let names = choices(a, Pane::Song);
    let name = |part: PatternOrScene| {
        names
            .iter()
            .find(|(p, _)| *p == part)
            .map_or_else(|| format!("{part:?}"), |(_, n)| n.clone())
    };
    let mut song: Vec<_> = collection
        .song
        .iter()
        .map(|v| {
            let row = collection
                .song_lanes
                .iter()
                .position(|l| *l == v.lane)
                .unwrap();
            (row, v.measure, name(v.part))
        })
        .collect();
    song.sort();
    let s = &a.patterns.workspace.song_cell;
    Outcome {
        song,
        cell: (s.row, s.measure, s.placing, s.choice.map(name)),
    }
}

/// Selection moves, placements, a return to the event list and back, and an undo.
const KEYS: [(Key, Modifiers); 13] = [
    (Key::Escape, Modifiers::NONE),
    (Key::K, Modifiers::NONE),
    (Key::F, Modifiers::NONE),
    (Key::L, Modifiers::NONE),
    (Key::F, Modifiers::NONE),
    (Key::L, Modifiers::NONE),
    (Key::Enter, Modifiers::NONE),
    (Key::J, Modifiers::NONE),
    (Key::F, Modifiers::NONE),
    (Key::Z, Modifiers::CTRL),
    (Key::Escape, Modifiers::NONE),
    (Key::Escape, Modifiers::NONE),
    (Key::K, Modifiers::NONE),
];

fn run(placed: bool) -> Vec<Outcome> {
    let (mut a, c) = if placed {
        placed_song()
    } else {
        let (a, c, ..) = verification_song();
        (a, c)
    };
    let id = if placed {
        dock()
    } else {
        Pane::Song.viewport()
    };
    KEYS.iter()
        .map(|&(k, m)| {
            key(&mut a, &c, id, k, m);
            outcome(&a)
        })
        .collect()
}

#[test]
fn the_same_keys_edit_the_song_alike_in_its_pane_and_its_window() {
    let window = run(false);
    let pane = run(true);
    for (i, (w, p)) in window.iter().zip(&pane).enumerate() {
        assert_eq!(w, p, "after {:?}", KEYS[i]);
    }
    // The keys did place and undo.
    assert_ne!(window[8].song, window[7].song);
    assert_eq!(window[9].song, window[7].song);
}

#[test]
fn leaving_the_placed_song_hands_the_keys_back_and_a_key_in_its_window_returns() {
    let (mut a, c) = placed_song();
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Song);
    a.patterns.workspace.song_cell.placing = false;
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Main);
    // Escape in the event list returns to the Song it came from.
    a.patterns.workspace.return_to = Some(Pane::Song);
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
}

#[test]
fn a_dialog_opened_in_the_pane_holds_its_dock_window() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "song" }, { view = "mixer" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    opened(&c, Pane::Song.viewport());
    key(&mut a, &c, dock(), Key::K, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Song);
    command(&mut a, &c, dock(), "scene.create");
    assert_eq!(
        a.patterns.workspace.dialog.as_ref().map(|d| d.view),
        Some(ViewKind::Song)
    );
    frame(&mut a, &c, dock(), vec![]);
    let field = egui::Id::new(("arrangement-field", "arrangement-name"));
    assert!(c.data(|d| d.get_temp::<egui::Rect>(field)).is_some());
    assert!(!opened(&c, Pane::Song.viewport()));
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
    assert_eq!(focus::focused(&a), ViewKind::Song);
    assert!(a.patterns.workspace.input_open());
    // The press outside took the text focus away.
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
    assert!(
        collection.scenes.iter().any(|s| s.name == "B"),
        "{:?}",
        collection
            .scenes
            .iter()
            .map(|s| (&s.name, s.meters.len()))
            .collect::<Vec<_>>()
    );
    command(&mut a, &c, dock(), "scene.create");
    assert!(a.patterns.workspace.input_open());
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
}

#[test]
fn play_opened_from_the_pane_shows_in_its_dock_window_over_disabled_panes() {
    let (mut a, c, ..) = verification_song();
    place(&mut a, r#"{ view = "song" }, { view = "mixer" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    a.patterns.workspace.song_cell.placing = true;
    key(&mut a, &c, dock(), Key::P, Modifiers::NONE);
    assert!(a.playback.open);
    assert_eq!(crate::view::commands::root_owner(&a), Screen::Play);
    // A new window is sized in its first frame and painted from the next.
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    let shapes = c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", dock())))
    });
    assert!(format!("{shapes:?}").contains("Song start"));
    // As in the main window, the pane beside the Song takes no press under the Play window.
    let mixer = egui::pos2(450.0, 580.0);
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
    assert_eq!(focus::focused(&a), ViewKind::Song);
    assert!(a.playback.open);
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.playback.open);
}
