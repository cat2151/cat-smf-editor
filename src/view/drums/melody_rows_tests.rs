use super::melody_tests::{frame, ordinary_pattern_in, plain, set_rows};
use super::*;
use cat_smf_editor::score::{
    chord_guide,
    harmony::{PitchClasses, Set},
    scene::arrangement::{self, PatternOrScene},
    step_notes,
};

/// A new song with a chord guide holding `chords` (480 ticks per quarter, 4/4).
fn song(chords: &[(u64, &str)]) -> cat_smf_editor::model::Music {
    let mut music = cat_smf_editor::edit::tracks::new_song();
    chord_guide::add(&mut music).unwrap();
    let guide = chord_guide::track(&music).unwrap();
    for &(tick, text) in chords {
        cat_smf_editor::edit::insert(
            &mut music.tracks[guide],
            chord_guide::event(tick, text).unwrap(),
        );
    }
    music
}

/// Every pitch whose class is one of `classes`, highest first, as the grid lists its rows.
fn pitches(classes: &[u8]) -> Vec<u8> {
    let set = PitchClasses::of(classes.iter().copied());
    (0..=127).rev().filter(|&p| set.contains(p)).collect()
}

fn shift_d(a: &mut App, ctx: &egui::Context) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key: Key::D,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        })
        .collect();
    frame(a, ctx, events);
}

fn opened(music: cat_smf_editor::model::Music, rows: Set) -> (App, egui::Context) {
    let mut a = ordinary_pattern_in(music);
    set_rows(&mut a, rows);
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift_d(&mut a, &ctx);
    assert!(is_open(&a) && melody::active(&a), "{:?}", a.load_error);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}

/// Whether the drawn cell of `pitch` at `step` is darkened as outside its step's harmony.
fn outside(ctx: &egui::Context, pitch: u8, step: usize) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(("melody-outside", pitch, step))))
        .unwrap_or_else(|| panic!("{pitch} at {step} was not drawn"))
}

#[test]
fn chord_rows_join_both_chords_of_the_measure_and_each_column_follows_its_beat() {
    // C on beats 1-2, F on beats 3-4.
    let (mut a, ctx) = opened(song(&[(0, "C"), (960, "F")]), Set::Chord);
    assert_eq!(melody::rows(&mut a), pitches(&[0, 4, 7, 5, 9]));
    for _ in 0..2 {
        frame(&mut a, &ctx, vec![]);
    }
    // Steps 1-8 sound under C, steps 9-16 under F.
    for step in [0, 7] {
        assert!(!outside(&ctx, 64, step) && !outside(&ctx, 60, step));
        assert!(outside(&ctx, 65, step) && outside(&ctx, 69, step));
    }
    for step in [8, 15] {
        assert!(outside(&ctx, 64, step) && outside(&ctx, 67, step));
        assert!(!outside(&ctx, 65, step) && !outside(&ctx, 60, step));
    }
    // Chromatic darkens nothing.
    plain(&mut a, &ctx, Key::Slash);
    assert_eq!(a.config.melody_rows(a.patterns.container), Set::Chromatic);
    frame(&mut a, &ctx, vec![]);
    assert!(!outside(&ctx, 65, 0) && !outside(&ctx, 61, 8));
}

#[test]
fn a_placed_note_keeps_its_row_outside_the_shown_set() {
    let (mut a, _ctx) = opened(song(&[(0, "C")]), Set::Chord);
    let track = a.track;
    a.edit_music(|m| step_notes::toggle(m, track, 61, 1, 4, 100))
        .unwrap();
    let rows = melody::rows(&mut a);
    assert!(rows.contains(&61));
    assert!(!rows.contains(&73) && !rows.contains(&62));
    assert_eq!(rows.len(), pitches(&[0, 4, 7]).len() + 1);
}

#[test]
fn slash_steps_through_the_four_sets_and_the_store_keeps_the_choice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let mut a = ordinary_pattern_in(song(&[(0, "Am")]));
    a.config = crate::store::config::Store::open(path.clone());
    // Nothing saved yet: Chord.
    assert_eq!(a.config.melody_rows(a.patterns.container), Set::Chord);
    let ctx = egui::Context::default();
    frame(&mut a, &ctx, vec![]);
    shift_d(&mut a, &ctx);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(melody::rows(&mut a), pitches(&[9, 0, 4]));
    // C4 is a tone of Am, so the cursor starts there.
    assert_eq!(melody::pitch(&a), 60);
    let expected = [
        (Set::Chromatic, (0..=127).rev().collect::<Vec<u8>>()),
        (Set::Scale, pitches(&[0, 2, 4, 5, 7, 9, 11])),
        (Set::Penta, pitches(&[0, 2, 4, 7, 9])),
        (Set::Chord, pitches(&[9, 0, 4])),
    ];
    for (set, rows) in expected {
        plain(&mut a, &ctx, Key::Slash);
        assert_eq!(a.config.melody_rows(a.patterns.container), set);
        assert_eq!(melody::rows(&mut a), rows, "{set:?}");
        assert_eq!(
            crate::store::config::Store::open(path.clone()).melody_rows(a.patterns.container),
            set
        );
    }
    // Chromatic on C#4, then Scale: C#4 is gone, and of C4 and D4 the lower takes the cursor.
    plain(&mut a, &ctx, Key::Slash);
    plain(&mut a, &ctx, Key::K);
    assert_eq!(melody::pitch(&a), 61);
    plain(&mut a, &ctx, Key::Slash);
    assert_eq!(melody::pitch(&a), 60);
    assert_eq!(
        crate::store::config::Store::open(path).melody_rows(a.patterns.container),
        Set::Scale
    );
}

#[test]
fn the_harmony_is_read_where_the_pattern_sounds_in_the_song() {
    // C in measure 1, F in measure 2 (tick 1920).
    let (mut a, ctx) = opened(song(&[(0, "C"), (1920, "F")]), Set::Chord);
    let id = crate::view::patterns::mode::editing(&a, a.track)
        .unwrap()
        .pattern;
    // Opened from the candidates: the Song cursor.
    a.patterns.workspace.song_cell.measure = 1;
    assert_eq!(melody::rows(&mut a), pitches(&[0, 4, 7]));
    a.patterns.workspace.song_cell.measure = 2;
    assert_eq!(melody::rows(&mut a), pitches(&[5, 9, 0]));
    // Opened from a placement in measure 2: that placement, wherever the Song cursor goes.
    a.edit_tracks(|m| {
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 2, Some(PatternOrScene::Pattern(id)))
    })
    .unwrap();
    crate::view::patterns::workspace::edit(
        &mut a,
        &ctx,
        crate::view::patterns::workspace::Pane::Song,
    );
    assert_eq!(a.load_error, None);
    a.patterns.workspace.song_cell.measure = 1;
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, id).unwrap(),
        1920
    );
    open(&mut a);
    assert!(is_open(&a) && melody::active(&a), "{:?}", a.load_error);
    assert_eq!(melody::rows(&mut a), pitches(&[5, 9, 0]));
}
