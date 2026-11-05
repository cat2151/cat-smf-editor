//! Which pattern a container shows, by every way into it.
use super::workspace::Pane;
use super::workspace_tests::{frame, key, verification_song};
use super::*;
use cat_smf_editor::score::{
    note_view::note_on,
    pattern::operations,
    scene::{
        self,
        arrangement::{self as grid, PatternOrScene},
    },
};
use eframe::egui::{self, Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;

fn notes(a: &App, track: usize) -> Vec<(u64, u8)> {
    a.loaded.as_ref().unwrap().visible().tracks[track]
        .events
        .iter()
        .filter(|e| note_on(e))
        .map(|e| (e.tick, e.data[0]))
        .collect()
}
/// The one note of the fixture's pattern, at the start of the shown timeline.
fn assert_shows(a: &App, id: u64, pitch: u8) {
    assert_eq!(a.patterns.editing.as_ref().map(|c| c.pattern), Some(id));
    assert_eq!(notes(a, a.track), vec![(0, pitch)]);
}
fn pitches(a: &App, first: u64, second: u64) -> (u8, u8) {
    let c =
        cat_smf_editor::score::pattern::Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let track = &a.loaded.as_ref().unwrap().music.tracks[a.track];
    let pitch = |id: u64| {
        let p = &c.patterns[&id];
        track
            .events
            .iter()
            .find(|e| note_on(e) && e.tick >= p.tick)
            .unwrap()
            .data[0]
    };
    (pitch(first), pitch(second))
}
/// The fixture stopped, showing `first`.
fn stopped() -> (App, egui::Context, u64, u64, (u8, u8)) {
    let (mut a, c, first, second, _) = verification_song();
    crate::play::transport::stop(&mut a);
    library::edit(&mut a, first).unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    let notes = pitches(&a, first, second);
    (a, c, first, second, notes)
}
/// Plays the Song to `seconds`: `first` sounds in measure 1, `second` from measure 2.
fn song_at(a: &mut App, c: &egui::Context, seconds: f64) {
    if a.playback.active.is_none() {
        crate::play::arrangement::start(a, None);
        crate::play::transport::advance(a);
    }
    a.audition.set_test_seconds(seconds);
    frame(a, c, ROOT, vec![]);
}

#[test]
fn every_way_into_a_container_shows_one_pattern() {
    let (mut a, c, first, second, (low, high)) = stopped();
    let container = a.track;
    a.select_track(0);
    a.select_track(container);
    assert_shows(&a, first, low);

    crate::cursor::split::go(&mut a, 0, 0);
    crate::cursor::split::go(&mut a, container, 1_000_000);
    assert_shows(&a, first, low);

    key(&mut a, &c, ROOT, Key::V, Modifiers::NONE);
    a.patterns.raw_context = None;
    key(&mut a, &c, ROOT, Key::V, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);

    a.track_view.cursor_tick = Some(1_000_000);
    a.selected_event = None;
    a.cursor_row = None;
    a.cursor_position = None;
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);
    assert!(crate::play::playback::location(&a).0 < 1920);

    a.patterns.editing = None;
    refresh(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);

    library::edit(&mut a, second).unwrap();
    workspace::mutate(&mut a, |m| {
        let mut collection = scene::read(m)?;
        collection.song.clear();
        collection.scenes.iter_mut().for_each(|s| s.cells.clear());
        scene::write(m, &collection)?;
        operations::delete(m, second)
    })
    .unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);
    assert_ne!(low, high);
}

#[test]
fn a_container_returns_to_the_pattern_last_shown_in_it() {
    let (mut a, _c, _, second, (_, high)) = stopped();
    let container = a.track;
    library::edit(&mut a, second).unwrap();
    a.select_track(0);
    a.select_track(container);
    assert_shows(&a, second, high);
}

#[test]
fn raw_shows_the_saved_track() {
    let (mut a, c, _, _, _) = stopped();
    key(&mut a, &c, ROOT, Key::V, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(a.patterns.editing.is_none());
    assert_eq!(notes(&a, a.track).len(), 2);
}

#[test]
fn a_running_song_shows_what_it_plays_and_stopping_stays_there() {
    let (mut a, c, first, second, (low, high)) = stopped();
    let candidate = a.patterns.workspace.song_cell.choice;
    song_at(&mut a, &c, 0.5);
    assert_shows(&a, first, low);
    song_at(&mut a, &c, 2.5);
    assert_shows(&a, second, high);
    assert_eq!(a.patterns.selected, Some(second));
    assert_eq!(
        crate::play::transport::display_tick_for(&a, a.track),
        Some(480)
    );
    assert!(a.playback.active.is_some());
    assert_eq!(a.patterns.workspace.song_cell.choice, candidate);
    crate::play::transport::stop(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, second, high);
}

#[test]
fn a_running_scene_shows_what_it_plays() {
    let (mut a, c, first, second, (low, high)) = stopped();
    let id = scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .scenes[0]
        .id;
    crate::play::arrangement::start(&mut a, Some(id));
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(0.5);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);
    a.audition.set_test_seconds(2.5);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, second, high);
}

#[test]
fn choosing_a_container_during_the_song_shows_what_it_plays_and_keeps_playing() {
    let (mut a, c, _, second, (_, high)) = stopped();
    let container = a.track;
    a.select_track(0);
    song_at(&mut a, &c, 2.5);
    a.select_track(container);
    assert_shows(&a, second, high);
    assert!(a.playback.active.is_some());
}

#[test]
fn a_silent_measure_keeps_the_shown_pattern() {
    let (mut a, c, _, second, (_, high)) = stopped();
    a.edit_tracks(|m| {
        let lane = scene::read(m)?.song_lanes[0];
        grid::place(m, None, lane, 9, Some(PatternOrScene::Pattern(second)))
    })
    .unwrap();
    library::edit(&mut a, second).unwrap();
    // Past the placements of measures 1 to 3.
    song_at(&mut a, &c, 12.0);
    assert_shows(&a, second, high);
}

#[test]
fn patterns_sounding_together_leave_the_shown_one_in_place() {
    let (mut a, c, first, second, (_, high)) = stopped();
    a.edit_tracks(|m| {
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 3, Some(PatternOrScene::Pattern(first)))
    })
    .unwrap();
    library::edit(&mut a, second).unwrap();
    song_at(&mut a, &c, 4.5);
    assert_shows(&a, second, high);
}

#[test]
fn an_open_input_holds_the_shown_pattern_until_it_closes() {
    let (mut a, c, first, second, (low, high)) = stopped();
    a.track_view.go_open = true;
    song_at(&mut a, &c, 2.5);
    assert_shows(&a, first, low);
    a.track_view.go_open = false;
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, second, high);
}

#[test]
fn a_held_pointer_holds_the_shown_pattern_until_it_is_released() {
    let (mut a, c, first, second, (low, high)) = stopped();
    let pos = egui::pos2(300.0, 300.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    frame(&mut a, &c, ROOT, vec![egui::Event::PointerMoved(pos)]);
    frame(&mut a, &c, ROOT, vec![button(true)]);
    song_at(&mut a, &c, 2.5);
    assert_shows(&a, first, low);
    frame(&mut a, &c, ROOT, vec![button(false)]);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, second, high);
}

#[test]
fn the_other_column_never_shows_a_container_timeline() {
    let (mut a, c, first, _, (low, _)) = stopped();
    let container = a.track;
    a.track_view.split = Some([container, 0]);
    a.track_view.active = 0;
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!mode::unprojected(&a, container));
    crate::cursor::split::focus(&mut a, 1);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(a.track, 0);
    assert!(mode::unprojected(&a, container));
    crate::cursor::split::focus(&mut a, 0);
    frame(&mut a, &c, ROOT, vec![]);
    assert_shows(&a, first, low);
}

#[test]
fn the_song_window_keeps_following_while_it_has_focus() {
    let (mut a, c, _, second, (_, high)) = stopped();
    crate::play::arrangement::start(&mut a, None);
    crate::play::transport::advance(&mut a);
    a.audition.set_test_seconds(2.5);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert_shows(&a, second, high);
}
