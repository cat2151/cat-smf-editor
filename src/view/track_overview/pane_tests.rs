//! The overview placed in a pane draws there instead of floating, and acts as the floating one does.
use super::*;
use crate::view::commands::owner;
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key};
use egui::Modifiers;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

/// The overview above a mixer in a dock window.
const DOCK: &str = r#"
[[window]]
title = "Dock"
rect = [0, 0, 1200, 800]
column = [{ view = "track_overview" }, { view = "mixer" }]
"#;
const UNDER_THE_LIST: &str = r#"
[root]
column = [{ view = "event_list" }, { view = "track_overview" }]
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Tracks,
    Song,
}
const KINDS: [Kind; 2] = [Kind::Tracks, Kind::Song];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    Floating,
    Dock,
    Root,
}

fn layout(place: Place) -> crate::store::layout::Spec {
    crate::store::layout::parse(match place {
        Place::Floating => "",
        Place::Dock => DOCK,
        Place::Root => UNDER_THE_LIST,
    })
    .unwrap()
}

/// The open overview of three tracks, or of a song with containers.
fn placed(kind: Kind, place: Place) -> App {
    let mut a = match kind {
        Kind::Tracks => {
            let mut a = super::tests::fixture();
            a.select_track(1);
            a
        }
        Kind::Song => {
            let (mut a, _, _, second, _) =
                crate::view::patterns::workspace_tests::verification_song();
            crate::view::patterns::workspace_tests::lengthen_song(&mut a, second, 8);
            a
        }
    };
    a.layout.spec = layout(place);
    open(&mut a);
    assert_eq!(is_song(&a), kind == Kind::Song);
    a
}

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

/// The window receiving the keys typed into the overview.
fn host(place: Place) -> egui::ViewportId {
    if place == Place::Dock {
        dock()
    } else {
        ROOT
    }
}

fn minimap(kind: Kind) -> egui::Id {
    use crate::view::timeline_controls::Screen;
    let screen = match kind {
        Kind::Tracks => Screen::TrackOverview,
        Kind::Song => Screen::TrackOverviewSong,
    };
    egui::Id::new((screen, "minimap"))
}

/// Draws one frame and tells whether the overview's timeline was drawn anywhere in it.
fn drawn(a: &mut App, c: &egui::Context, kind: Kind) -> bool {
    c.data_mut(|d| d.remove::<egui::Rect>(minimap(kind)));
    frame(a, c, ROOT, vec![]);
    c.data(|d| d.get_temp::<egui::Rect>(minimap(kind)).is_some())
}

/// The overview's floating window in the main window, maximized or not.
fn floating(c: &egui::Context) -> bool {
    ["Track Overview", "track-overview-maximized"]
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
fn each_placed_overview_draws_in_its_pane_and_floats_no_window() {
    for kind in KINDS {
        for place in [Place::Dock, Place::Root] {
            let mut a = placed(kind, place);
            let c = context();
            frame(&mut a, &c, ROOT, vec![]);
            assert!(drawn(&mut a, &c, kind), "{kind:?} {place:?}");
            assert!(!floating(&c), "{kind:?} {place:?}");
            assert_eq!(focus::focused(&a), ViewKind::TrackOverview);
            assert_eq!(owner(&a, &c), Screen::Overview);
            // Opening it keeps the window holding it shown.
            assert!(visible(&c, host(place)), "{kind:?} {place:?}");
            // Taken out of the layout, the overview floats again.
            a.layout.spec = layout(Place::Floating);
            assert!(drawn(&mut a, &c, kind), "{kind:?} {place:?}");
            assert!(floating(&c), "{kind:?} {place:?}");
        }
    }
}

#[test]
fn the_closed_overview_leaves_its_pane_empty_and_takes_no_focus() {
    let mut a = placed(Kind::Tracks, Place::Dock);
    a.track_overview.open = false;
    let c = context();
    assert!(!drawn(&mut a, &c, Kind::Tracks));
    // The key reaches the mixer below, passing over the empty pane.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // A click on the event list hands the focus back before the view opens from there.
    focus::set(&mut a, ViewKind::EventList);
    open(&mut a);
    assert!(drawn(&mut a, &c, Kind::Tracks));
    assert_eq!(focus::focused(&a), ViewKind::TrackOverview);
    // Esc closes the overview, and the keys go back to the event list.
    press(&mut a, &c, dock(), Key::Escape);
    assert!(!a.track_overview.open);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    assert!(!drawn(&mut a, &c, Kind::Tracks));
}

#[test]
fn moving_to_another_pane_closes_the_overview() {
    let mut a = placed(Kind::Tracks, Place::Dock);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    press(&mut a, &c, dock(), Key::F6);
    assert!(!a.track_overview.open);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // The emptied pane is passed over.
    press(&mut a, &c, dock(), Key::F6);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
}

#[test]
fn the_overview_placed_in_the_main_window_takes_a_click() {
    let mut a = placed(Kind::Tracks, Place::Root);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    let rect = c
        .read_response(egui::Id::new(("track-overview-pane", 1)))
        .unwrap()
        .rect;
    // 0.2 px per tick: x 400 is tick 2000 inside the second measure.
    let at = egui::pos2(rect.left() + 400.0, rect.center().y);
    frame(&mut a, &c, ROOT, vec![egui::Event::PointerMoved(at)]);
    for pressed in [true, false] {
        frame(
            &mut a,
            &c,
            ROOT,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    assert_eq!(a.track_overview.cell, (1, 1920));
    let prepared = a.playback.pending.as_ref().unwrap();
    assert!((prepared.active.start_seconds - 2.0).abs() < 1e-9);
    assert!(a.track_overview.open);
}

#[test]
fn play_opened_from_the_overview_shows_in_its_window() {
    for (kind, title) in [
        (Kind::Tracks, "Start → earlier · End ↓ later"),
        (Kind::Song, "Enter — Song"),
    ] {
        let mut a = placed(kind, Place::Dock);
        let c = context();
        frame(&mut a, &c, ROOT, vec![]);
        press(&mut a, &c, dock(), Key::P);
        frame(&mut a, &c, ROOT, vec![]);
        assert_eq!(owner(&a, &c), Screen::Play, "{kind:?}");
        assert!(visible(&c, dock()), "{kind:?}");
        assert!(shows_text(&c, dock(), title), "{kind:?}");
        assert!(!shows_text(&c, ROOT, title), "{kind:?}");
        press(&mut a, &c, dock(), Key::Escape);
        assert!(!a.playback.open, "{kind:?}");
        assert_eq!(owner(&a, &c), Screen::Overview, "{kind:?}");
    }
}

/// What the user can observe after each key of one sequence.
#[derive(Debug, PartialEq)]
struct Step {
    owner: Screen,
    open: bool,
    cell: (usize, u64),
    track: usize,
    cursor: Option<u64>,
    play: bool,
    /// Mode, start, end and the number of containers of the requested playback.
    playback: Option<(String, f64, f64, Option<usize>)>,
    error: Option<String>,
    /// The Song version's jump opens the pattern under the cell.
    editing: bool,
}

fn observe(a: &App, c: &egui::Context) -> Step {
    Step {
        owner: owner(a, c),
        open: a.track_overview.open,
        cell: a.track_overview.cell,
        track: a.track,
        cursor: a.track_view.cursor_tick,
        play: a.playback.open,
        playback: a
            .playback
            .pending
            .as_ref()
            .map(|p| &p.active)
            .or(a.playback.active.as_ref())
            .map(|p| {
                (
                    format!("{:?}", p.mode),
                    p.start_seconds,
                    p.end,
                    p.containers.as_ref().map(Vec::len),
                )
            }),
        error: a.load_error.clone(),
        editing: a.patterns.editing.is_some(),
    }
}

const SEQUENCE: [Key; 12] = [
    Key::L,
    Key::L,
    Key::H,
    Key::K,
    Key::J,
    Key::Enter,
    Key::T,
    Key::Space,
    Key::H,
    Key::P,
    Key::Escape,
    Key::G,
];

fn run(kind: Kind, place: Place) -> Vec<Step> {
    let mut a = placed(kind, place);
    let c = context();
    let id = host(place);
    frame(&mut a, &c, ROOT, vec![]);
    let mut steps = vec![observe(&a, &c)];
    for k in SEQUENCE {
        press(&mut a, &c, id, k);
        steps.push(observe(&a, &c));
    }
    steps
}

#[test]
fn a_placed_overview_moves_plays_and_jumps_as_the_floating_one() {
    for kind in KINDS {
        let floating = run(kind, Place::Floating);
        let n = floating.len();
        // The keys move the cell, Enter and T start playback, P opens the play menu and G jumps.
        assert_ne!(floating[0].cell, floating[3].cell, "{kind:?}");
        assert!(floating[6].playback.is_some(), "{kind:?}");
        assert_ne!(floating[6].playback, floating[7].playback, "{kind:?}");
        assert!(floating[n - 3].play, "{kind:?}");
        assert!(floating[n - 2].open && !floating[n - 1].open, "{kind:?}");
        assert_eq!(floating[n - 1].editing, kind == Kind::Song, "{kind:?}");
        for place in [Place::Dock, Place::Root] {
            let pane = run(kind, place);
            assert_eq!(pane.len(), n);
            for (i, (pane, floating)) in pane.iter().zip(&floating).enumerate() {
                assert_eq!(pane, floating, "{kind:?} {place:?} step {i}");
            }
        }
    }
}
