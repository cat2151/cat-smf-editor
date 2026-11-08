//! The grid placed in a pane draws there instead of floating, and edits as the floating one does.
use super::window::Kind;
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key, text};
use egui::Modifiers;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
const KINDS: [Kind; 3] = [Kind::Drums, Kind::Kit, Kind::Melody];

/// The grid above a mixer in a dock window.
const DOCK: &str = r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "step_sequencer" }, { view = "mixer" }]
"#;
const BESIDE_THE_LIST: &str = r#"
[root]
row = [{ view = "event_list" }, { view = "step_sequencer" }]
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Floating,
    Dock,
    Root,
}

fn fixture(kind: Kind) -> App {
    match kind {
        Kind::Drums => super::tests::fixture(),
        Kind::Kit => {
            let mut a = super::kit_edit_tests::fixture(4);
            open(&mut a);
            a
        }
        Kind::Melody => super::melody_tests::fixture(),
    }
}

/// The grid as its own input opens it in `place`.
fn placed(kind: Kind, place: Place) -> App {
    let mut a = fixture(kind);
    a.layout.spec = crate::store::layout::parse(match place {
        Place::Floating => "",
        Place::Dock => DOCK,
        Place::Root => BESIDE_THE_LIST,
    })
    .unwrap();
    take_keys(&mut a);
    a
}

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

/// The window receiving the keys typed into the grid.
fn host(place: Place) -> egui::ViewportId {
    if place == Place::Dock {
        dock()
    } else {
        ROOT
    }
}

/// A test-only rectangle each grid records while drawing its cells.
fn marker(kind: Kind) -> egui::Id {
    egui::Id::new(match kind {
        Kind::Drums => "drum-grid-bounds",
        Kind::Kit => "kit-clip",
        Kind::Melody => "melody-clip",
    })
}

/// Draws one frame and tells whether the grid's cells were drawn anywhere in it.
fn drawn(a: &mut App, c: &egui::Context, kind: Kind) -> bool {
    let id = marker(kind);
    c.data_mut(|d| d.remove::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(id));
    c.data_mut(|d| d.remove::<egui::Rect>(id));
    frame(a, c, ROOT, vec![]);
    c.data(|d| {
        d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(id)
            .is_some()
            || d.get_temp::<egui::Rect>(id).is_some()
    })
}

/// The grid's floating window in the main window.
fn floating(c: &egui::Context, kind: Kind) -> bool {
    c.memory(|m| m.area_rect(kind.id()).is_some())
}

fn visible(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", id))))
        .unwrap_or(true)
}

fn shapes(c: &egui::Context, id: egui::ViewportId) -> Vec<egui::epaint::ClippedShape> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
}

fn press(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key) {
    key(a, c, id, k, Modifiers::NONE);
}

#[test]
fn each_placed_grid_draws_in_its_pane_and_floats_no_window() {
    for kind in KINDS {
        for place in [Place::Dock, Place::Root] {
            let mut a = placed(kind, place);
            let c = context();
            frame(&mut a, &c, ROOT, vec![]);
            assert!(drawn(&mut a, &c, kind), "{place:?}");
            assert!(!floating(&c, kind), "{place:?}");
            assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
            assert_eq!(owner(&a, &c), Screen::Drums);
            // Taken out of the layout, the grid floats again.
            a.layout.spec = crate::store::layout::parse("").unwrap();
            assert!(drawn(&mut a, &c, kind), "{place:?}");
            assert!(floating(&c, kind), "{place:?}");
        }
    }
}

#[test]
fn the_empty_pane_takes_no_focus_until_the_grid_opens() {
    let mut a = placed(Kind::Drums, Place::Dock);
    close(&mut a);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!drawn(&mut a, &c, Kind::Drums));
    // The key reaches the mixer below, passing over the empty pane.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // A click on the event list hands the focus back before the view opens from there.
    focus::set(&mut a, ViewKind::EventList);
    open(&mut a);
    assert!(drawn(&mut a, &c, Kind::Drums));
    assert!(!floating(&c, Kind::Drums));
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    // Esc closes the grid, and the keys go back to the event list.
    press(&mut a, &c, dock(), Key::Escape);
    assert!(!is_open(&a));
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert!(!drawn(&mut a, &c, Kind::Drums));
}

#[test]
fn a_key_in_the_dock_window_reaches_the_mixer_below_the_empty_grid_pane() {
    let mut a = placed(Kind::Drums, Place::Dock);
    close(&mut a);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::ArrowDown);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(owner(&a, &c), Screen::Mixer);
}

#[test]
fn moving_to_another_pane_keeps_the_grid_open() {
    let mut a = placed(Kind::Drums, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::F6);
    assert!(is_open(&a));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(owner(&a, &c), Screen::Mixer);
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    assert_eq!(owner(&a, &c), Screen::Drums);
}

/// What the user can observe after each key of one sequence.
#[derive(Debug, PartialEq)]
struct Step {
    owner: Screen,
    open: bool,
    cursor: (usize, usize, u64),
    input: bool,
    bytes: Vec<u8>,
}

/// The SMF bytes, or for a pattern the notes of its body: patterns get random ids.
fn song(a: &App, kind: Kind) -> Vec<u8> {
    let loaded = a.loaded.as_ref().unwrap();
    if !matches!(kind, Kind::Melody) {
        return cat_smf_editor::smf_io::encode(&loaded.music).unwrap();
    }
    cat_smf_editor::score::step_notes::notes(&loaded.visible().tracks[a.track])
        .into_iter()
        .flat_map(|n| {
            [n.pitch, n.velocity]
                .into_iter()
                .chain(n.start.to_le_bytes())
                .chain(n.end.to_le_bytes())
        })
        .collect()
}

fn observe(a: &App, c: &egui::Context, kind: Kind) -> Step {
    Step {
        owner: owner(a, c),
        open: is_open(a),
        cursor: a.drum_editor.command_target(),
        input: a.drum_editor.input.is_some() || a.drum_editor.kit.has_input(),
        bytes: song(a, kind),
    }
}

fn sequence(kind: Kind) -> Vec<(Key, Modifiers)> {
    let none = Modifiers::NONE;
    let mut keys = vec![
        (Key::F, none),
        (Key::L, none),
        (Key::L, none),
        (Key::F, none),
        (Key::J, none),
        (Key::F, none),
        (Key::Z, Modifiers::CTRL),
    ];
    match kind {
        // To the Velocity field, then its input.
        Kind::Drums => {
            keys.extend(std::iter::repeat_n((Key::L, none), 18));
            keys.push((Key::Enter, none));
            keys.extend(std::iter::repeat_n((Key::Backspace, none), 3));
        }
        Kind::Kit => {}
        Kind::Melody => keys.push((Key::L, Modifiers::SHIFT)),
    }
    keys
}

fn run(kind: Kind, place: Place) -> Vec<Step> {
    let mut a = placed(kind, place);
    let c = context();
    let id = host(place);
    frame(&mut a, &c, ROOT, vec![]);
    let mut steps = vec![observe(&a, &c, kind)];
    for (k, m) in sequence(kind) {
        key(&mut a, &c, id, k, m);
        frame(&mut a, &c, ROOT, vec![]);
        steps.push(observe(&a, &c, kind));
    }
    if matches!(kind, Kind::Drums) {
        // The input stays in the window holding the grid, which stays shown and usable.
        assert!(a.drum_editor.input.is_some(), "{place:?}");
        assert!(visible(&c, id), "{place:?}");
        text(&mut a, &c, id, "40");
        press(&mut a, &c, id, Key::Enter);
        steps.push(observe(&a, &c, kind));
        assert!(a.drum_editor.input.is_none(), "{place:?}");
    }
    press(&mut a, &c, id, Key::Escape);
    steps.push(observe(&a, &c, kind));
    steps
}

#[test]
fn a_placed_grid_edits_as_the_floating_one() {
    for kind in KINDS {
        let floating = run(kind, Place::Floating);
        // `F` edits the song and Ctrl+Z (the 7th key) takes an edit back.
        assert_ne!(floating[0].bytes, floating[1].bytes);
        assert_ne!(floating[6].bytes, floating[7].bytes);
        assert_eq!(floating[7].bytes, floating[4].bytes);
        if matches!(kind, Kind::Drums) {
            let n = floating.len();
            assert!(floating[n - 3].input);
            assert_ne!(floating[n - 3].bytes, floating[n - 2].bytes);
        }
        assert!(!floating.last().unwrap().open);
        for place in [Place::Dock, Place::Root] {
            let pane = run(kind, place);
            assert_eq!(pane.len(), floating.len());
            for (i, (pane, floating)) in pane.iter().zip(&floating).enumerate() {
                assert_eq!(pane, floating, "{place:?} step {i}");
            }
        }
    }
}

#[test]
fn the_grid_pane_leaves_the_style_of_the_other_panes_alone() {
    let label = |open: bool| {
        let mut a = placed(Kind::Drums, Place::Dock);
        if !open {
            close(&mut a);
        }
        let c = context();
        let original = c.style();
        for _ in 0..3 {
            frame(&mut a, &c, ROOT, vec![]);
        }
        assert_eq!(is_open(&a), open);
        // The grid holds the keys, which the floating window shows in its title colors.
        if open {
            assert!(super::title::active(&a, &c));
        }
        assert_eq!(c.style(), original);
        shapes(&c, dock()).iter().find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "Master auto gain" => Some(format!("{t:?}")),
            _ => None,
        })
    };
    assert_eq!(label(true).expect("mixer pane"), label(false).unwrap());
}

#[test]
fn the_kit_note_picker_opens_in_the_dock_window_and_closes() {
    let mut a = placed(Kind::Kit, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::N);
    assert!(a.drum_editor.kit.has_input());
    c.data_mut(|d| d.remove::<egui::Rect>(egui::Id::new("kit-picker-clip")));
    frame(&mut a, &c, ROOT, vec![]);
    assert!(visible(&c, dock()));
    assert!(c.data(|d| d
        .get_temp::<egui::Rect>(egui::Id::new("kit-picker-clip"))
        .is_some()));
    assert!(!c.memory(|m| m.area_rect(egui::Id::new("kit-note-picker")).is_some()));
    // The picker replaces the grid until it closes.
    assert!(!drawn(&mut a, &c, Kind::Kit));
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
    press(&mut a, &c, dock(), Key::Escape);
    assert!(!a.drum_editor.kit.has_input());
    assert!(is_open(&a));
    assert!(drawn(&mut a, &c, Kind::Kit));
}

#[test]
fn the_track_creation_dialog_shows_in_the_dock_window() {
    let note = |tick, status, velocity| cat_smf_editor::model::Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![60, velocity],
    };
    let mut music = cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![
            cat_smf_editor::model::Track { events: vec![] },
            cat_smf_editor::model::Track {
                events: vec![note(0, 0x90, 100), note(480, 0x80, 0)],
            },
        ],
    };
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[1]).unwrap();
    let mut a = crate::view::tests::app(music);
    a.select_track(1);
    a.layout.spec = crate::store::layout::parse(DOCK).unwrap();
    open(&mut a);
    assert!(a.drum_editor.confirm_create);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(owner(&a, &c), Screen::DrumCreate);
    assert!(visible(&c, dock()));
    assert!(shapes(&c, dock()).iter().any(|s| matches!(
        &s.shape,
        egui::Shape::Text(t) if t.galley.text() == "Create a new drum track?"
    )));
    press(&mut a, &c, dock(), Key::Escape);
    assert!(!is_open(&a));
}

#[test]
fn the_docked_melody_grid_opens_on_the_pattern_notes_and_keeps_its_scroll() {
    let mut a = super::melody_tests::ordinary_pattern();
    let track = a.track;
    a.edit_music(|m| cat_smf_editor::edit::mml::insert(m, track, 0, 0, "o4ceg").map(|_| ()))
        .unwrap();
    a.layout.spec = crate::store::layout::parse(DOCK).unwrap();
    open(&mut a);
    let c = context();
    for _ in 0..4 {
        // The scroll area of the pane below must not overwrite the grid's.
        frame(&mut a, &c, ROOT, vec![]);
    }
    let clip: egui::Rect = c.data(|d| d.get_temp(marker(Kind::Melody))).unwrap();
    for pitch in [48, 52, 55] {
        let cell: Option<egui::Rect> =
            c.data(|d| d.get_temp(egui::Id::new(("melody-cell", pitch, 0usize))));
        assert!(
            cell.is_some_and(|r| clip.contains_rect(r)),
            "{pitch}: {cell:?} in {clip:?}"
        );
    }
}

#[test]
fn the_grid_the_app_closed_with_stays_open_beside_the_resumed_keyboard() {
    let mut a = super::melody_tests::ordinary_pattern();
    a.layout.spec = crate::store::layout::parse(
        r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "song" }, { view = "step_sequencer" }, { view = "keyboard" }]
"#,
    )
    .unwrap();
    open(&mut a);
    a.config.phrase_loop = true;
    a.config.keyboard_focused = false;
    crate::view::recording::resume(&mut a);
    let c = context();
    // The keyboard waits for the place of the window holding the Song.
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("window-geometry", dock())),
            [0.0_f32, 0.0, 1200.0, 800.0],
        )
    });
    for _ in 0..4 {
        frame(&mut a, &c, ROOT, vec![]);
    }
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(is_open(&a));
    assert_eq!(focus::focused(&a), ViewKind::StepSequencer);
}
