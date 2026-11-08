//! With the Song in a pane, the keyboard opens by itself beside the window holding that pane,
//! as it does beside the Song's own window.
use super::*;
use crate::view::dock::window;

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
const FRAMES: usize = 5;

/// Where the Song is drawn.
#[derive(Clone, Copy, Debug)]
enum Place {
    OwnWindow,
    Dock,
    Root,
}

impl Place {
    fn viewport(self) -> egui::ViewportId {
        match self {
            Place::OwnWindow => Pane::Song.viewport(),
            Place::Dock => window::viewport(0),
            Place::Root => ROOT,
        }
    }
}

/// A song with patterns, the Song placed at `place` and every window on the screen.
fn start(place: Place, resuming: bool) -> (App, egui::Context) {
    let (mut a, c, ..) = verification_song();
    let layout = match place {
        Place::OwnWindow => String::new(),
        Place::Dock => r#"
[[window]]
title = "Dock 1"
rect = [1920, 0, 1600, 900]
column = [
  { view = "song", height = 2 },
  { view = "mixer" },
]
"#
        .into(),
        Place::Root => r#"
[root]
row = [{ view = "event_list" }, { view = "song" }]
"#
        .into(),
    };
    a.layout.spec = crate::store::layout::parse(&layout).unwrap();
    a.config.phrase_loop = resuming;
    crate::view::recording::resume(&mut a);
    for (id, rect) in [
        (ROOT, [0.0_f32, 0.0, 1500.0, 900.0]),
        (Pane::Song.viewport(), [460.0, 520.0, 760.0, 520.0]),
        (window::viewport(0), [1920.0, 0.0, 1600.0, 900.0]),
    ] {
        c.data_mut(|d| d.insert_temp(egui::Id::new(("window-geometry", id)), rect));
    }
    (a, c)
}

fn opens(place: Place, resuming: bool) -> bool {
    let (mut a, c) = start(place, resuming);
    assert!(!a.patterns.recording.is_open());
    focus(&c, place.viewport());
    for _ in 0..FRAMES {
        frame(&mut a, &c, ROOT, vec![]);
        assert!(a.load_error.is_none(), "{:?}", a.load_error);
    }
    a.patterns.recording.is_open()
}

#[test]
fn the_keyboard_opens_beside_the_songs_own_window() {
    assert!(opens(Place::OwnWindow, false));
    assert!(opens(Place::OwnWindow, true));
}

#[test]
fn the_keyboard_opens_with_the_song_in_a_dock_window() {
    assert!(opens(Place::Dock, false));
    assert!(opens(Place::Dock, true));
}

/// On the real screen the main window can keep the focus all through startup, the window
/// showing the Song never getting it.
fn opens_while_the_main_window_keeps_the_focus(place: Place) -> bool {
    let (mut a, c) = start(place, true);
    focus(&c, ROOT);
    for _ in 0..60 {
        frame(&mut a, &c, ROOT, vec![]);
        assert!(a.load_error.is_none(), "{:?}", a.load_error);
    }
    a.patterns.recording.is_open()
}

#[test]
fn a_resumed_keyboard_opens_beside_the_songs_own_window_while_the_main_window_keeps_the_focus() {
    assert!(opens_while_the_main_window_keeps_the_focus(
        Place::OwnWindow
    ));
}

#[test]
fn a_resumed_keyboard_opens_beside_the_dock_window_while_the_main_window_keeps_the_focus() {
    assert!(opens_while_the_main_window_keeps_the_focus(Place::Dock));
}

#[test]
fn the_keyboard_opens_with_the_song_in_the_main_window() {
    assert!(opens(Place::Root, false));
    assert!(opens(Place::Root, true));
}
