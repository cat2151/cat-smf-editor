//! Every view, alone in a dock window, draws there, takes the keys typed into it and hides
//! behind the palette.
use super::*;
use crate::view::patterns::workspace::{self, Pane};
use crate::view::patterns::workspace_tests::verification_song;

pub(super) fn placing(name: &str) -> Result<crate::store::layout::Spec, String> {
    crate::store::layout::parse(&format!(
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"Dock\"\nrect = [0, 0, 900, 700]\nview = \"{name}\""
    ))
}

pub(super) fn kind(name: &str) -> ViewKind {
    match crate::store::layout::parse(&format!("[root]\nview = \"{name}\""))
        .unwrap()
        .root
    {
        Some(Node::View(view)) => view,
        other => panic!("{name}: {other:?}"),
    }
}

/// An app in which `view` has something to show.
pub(super) fn showing(view: ViewKind) -> (App, egui::Context) {
    let song = || {
        let (a, c, ..) = verification_song();
        (a, c)
    };
    match view {
        ViewKind::Score => {
            let (mut a, c) = song();
            a.note_mode = true;
            (a, c)
        }
        ViewKind::Analysis => {
            let (mut a, c) = song();
            crate::view::analysis::toggle(&mut a);
            (a, c)
        }
        ViewKind::Scene => {
            let (mut a, c, .., scene) = verification_song();
            workspace::open_scene(&mut a, scene).unwrap();
            (a, c)
        }
        ViewKind::TrackOverview => {
            let (mut a, c) = song();
            crate::view::track_overview::open(&mut a);
            (a, c)
        }
        ViewKind::StepSequencer => (crate::view::drums::tests::fixture(), context()),
        ViewKind::ParameterCurve => (crate::view::parameter_editor::tests::app(), context()),
        ViewKind::EventList
        | ViewKind::PianoRoll
        | ViewKind::TempoCurve
        | ViewKind::Mixer
        | ViewKind::Song
        | ViewKind::Keyboard
        | ViewKind::Piano
        | ViewKind::Help => song(),
    }
}

/// The dock window's shapes, without the focus frame, which an empty pane gets, too.
fn shapes(c: &egui::Context) -> usize {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", dock())))
    })
    .unwrap_or_default()
    .iter()
    .filter(
        |s| !matches!(&s.shape, egui::Shape::Rect(r) if r.stroke.color == crate::view::ACTIVE_PANE),
    )
    .count()
}

/// The shapes of a dock window whose only pane is empty: a closed step sequencer.
fn empty() -> usize {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    a.layout.spec = placing("step_sequencer").unwrap();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!crate::view::drums::is_open(&a));
    shapes(&c)
}

#[test]
fn every_view_alone_in_a_dock_window_draws_takes_its_keys_and_hides_behind_the_palette() {
    let empty = empty();
    let names = crate::store::layout::tests::view_names();
    assert_eq!(names.len(), 14, "{names:?}");
    for name in &names {
        let view = kind(name);
        if view == ViewKind::EventList {
            assert!(placing(name).is_err());
            continue;
        }
        let (mut a, c) = showing(view);
        a.layout.spec = placing(name).unwrap();
        frame(&mut a, &c, ROOT, vec![]);
        if matches!(view, ViewKind::Keyboard | ViewKind::Piano) {
            key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
            assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
        }
        frame(&mut a, &c, ROOT, vec![]);
        frame(&mut a, &c, ROOT, vec![]);
        assert_eq!(visible(&c), Some(true), "{view:?}");
        let drawn = shapes(&c);
        assert!(drawn > empty, "{view:?}: {drawn} shapes, empty {empty}");

        // A key typed into the window gives its pane the focus; Help and the piano never take it.
        key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
        let focused = if matches!(view, ViewKind::Help | ViewKind::Piano) {
            ViewKind::EventList
        } else {
            view
        };
        assert_eq!(focus::focused(&a), focused, "{view:?}");
        assert_eq!(
            crate::view::commands::root_owner(&a),
            focus::screen(focused),
            "{view:?}"
        );

        // The palette, open from every view, hides the window, and closing it brings the view
        // back with the focus kept.
        key(&mut a, &c, ROOT, Key::P, Modifiers::CTRL | Modifiers::SHIFT);
        assert!(a.palette.is_open(), "{view:?}");
        frame(&mut a, &c, ROOT, vec![]);
        assert_eq!(visible(&c), Some(false), "{view:?}");
        key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
        assert!(!a.palette.is_open(), "{view:?}");
        frame(&mut a, &c, ROOT, vec![]);
        frame(&mut a, &c, ROOT, vec![]);
        assert_eq!(visible(&c), Some(true), "{view:?}");
        assert_eq!(focus::focused(&a), focused, "{view:?}");
        let back = shapes(&c);
        assert!(back > empty, "{view:?}: {back} shapes, empty {empty}");
    }
}
