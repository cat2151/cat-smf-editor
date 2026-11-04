//! The step the drum grid frames as sounding under Pattern and Song playback.
use super::acceptance_tests::{drum_on_song, framed_step};
use super::*;
use crate::play::{playback::Mode, transport};
use cat_smf_editor::score::{
    audition_timing::TempoMap,
    pattern::Catalog,
    scene::{
        self,
        arrangement::{self, PatternOrScene},
    },
};

fn placed_at(a: &App, id: u64) -> u64 {
    arrangement::occurrences(&a.loaded.as_ref().unwrap().music, None)
        .unwrap()
        .0
        .into_iter()
        .find_map(|(p, at)| (p == id).then_some(at))
        .unwrap()
}

fn step_in_song(a: &mut App, tick: u64) -> Option<usize> {
    let tempo = TempoMap::new(&a.loaded.as_ref().unwrap().music, 0);
    crate::play::arrangement::start(a, None);
    transport::advance(a);
    a.audition.set_test_seconds(tempo.seconds_at(tick) + 0.001);
    assert_eq!(
        a.playback.active.as_ref().map(|x| x.mode),
        Some(Mode::Arrangement(None))
    );
    framed_step(a)
}

fn step_in_pattern(a: &mut App, id: u64, tick: u64) -> Option<usize> {
    let tempo = TempoMap::constant(480, a.patterns.tempo.0).unwrap();
    preview::start(a, id).unwrap();
    transport::advance(a);
    a.audition.set_test_seconds(tempo.seconds_at(tick) + 0.001);
    assert_eq!(
        a.playback.active.as_ref().map(|x| x.mode),
        Some(Mode::Pattern(id))
    );
    framed_step(a)
}

#[test]
fn pattern_playback_frames_the_step_of_the_edited_pattern_only() {
    let (mut a, _, _, id) = drum_on_song();
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let other = *catalog.patterns.keys().find(|&&p| p != id).unwrap();
    assert_eq!(step_in_pattern(&mut a, id, 5 * 120), Some(5));
    assert_eq!(step_in_pattern(&mut a, id, 0), Some(0));
    assert_eq!(step_in_pattern(&mut a, other, 5 * 120), None);
}

#[test]
fn moving_the_placement_moves_the_step_song_playback_frames() {
    let (mut a, _, _, id) = drum_on_song();
    let before = placed_at(&a, id);
    assert_eq!(step_in_song(&mut a, before + 5 * 120), Some(5));
    a.edit_tracks(|m| {
        let placement = scene::read(m)?
            .song
            .into_iter()
            .find(|p| p.part == PatternOrScene::Pattern(id))
            .unwrap();
        arrangement::place(m, None, placement.lane, placement.measure, None)?;
        arrangement::place(
            m,
            None,
            placement.lane,
            placement.measure + 2,
            Some(placement.part),
        )
    })
    .unwrap();
    let after = placed_at(&a, id);
    assert!(after > before + 8 * 120, "{before} -> {after}");
    assert_eq!(step_in_song(&mut a, after + 5 * 120), Some(5));
    assert_eq!(step_in_song(&mut a, before + 5 * 120), None);
}

#[test]
fn drum_grid_keyboard_edits_hold_pattern_notes_until_the_next_hit() {
    use crate::view::palette::tests::press;
    use cat_smf_editor::score::note_view::{note_on, NoteView};
    use eframe::egui::Key;

    let (mut a, ctx, drum, id) = drum_on_song();
    press(&mut a, &ctx, Key::Enter);
    for _ in 0..4 {
        press(&mut a, &ctx, Key::ArrowRight);
    }
    press(&mut a, &ctx, Key::Enter);
    let durations = |a: &App| {
        let music = &a.loaded.as_ref().unwrap().music;
        let catalog = Catalog::read(music).unwrap();
        let p = &catalog.patterns[&id];
        let events = cat_smf_editor::score::pattern::body::extract(
            &music.tracks[drum].events,
            p.range().unwrap(),
        )
        .unwrap();
        let notes = NoteView::new(&events);
        events
            .iter()
            .enumerate()
            .filter(|(_, e)| note_on(e))
            .map(|(i, _)| notes.durations[i].unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(durations(&a), vec![479, 1440]);
    assert!(preview::prepare(&a, id).is_ok());
    press(&mut a, &ctx, Key::Enter);
    assert_eq!(durations(&a), vec![1920]);
    a.undo_music().unwrap();
    assert_eq!(durations(&a), vec![479, 1440]);
    press(&mut a, &ctx, Key::Escape);
    assert!(!crate::view::drums::is_open(&a));
}

fn assert_marker(a: &App, local: Option<u64>) {
    let marker = transport::marker_for(a, a.track);
    let expected = local.map(|tick| {
        (0..a.loaded.as_ref().unwrap().display_rows[a.track].len())
            .rfind(|&row| crate::cursor::split::row_tick(a, a.track, row) <= tick)
            .unwrap()
    });
    assert_eq!(marker, expected);
}

fn add_hits(a: &mut App, ctx: &eframe::egui::Context) {
    use crate::view::palette::tests::press;
    use eframe::egui::Key;
    press(a, ctx, Key::Enter);
    for _ in 0..5 {
        press(a, ctx, Key::ArrowRight);
    }
    press(a, ctx, Key::Enter);
}

#[test]
fn both_markers_follow_repeated_song_occurrences_independent_of_song_selection() {
    let (mut a, ctx, _, id) = drum_on_song();
    add_hits(&mut a, &ctx);
    let first = placed_at(&a, id);
    a.edit_tracks(|m| {
        let placement = scene::read(m)?
            .song
            .into_iter()
            .find(|p| p.part == PatternOrScene::Pattern(id))
            .unwrap();
        arrangement::place(m, None, placement.lane, 7, Some(placement.part))
    })
    .unwrap();
    let other = *Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .keys()
        .find(|&&p| p != id)
        .unwrap();
    a.patterns.workspace.song_cell.choice = Some(PatternOrScene::Pattern(other));
    for at in [first, 6 * 1920] {
        assert_eq!(step_in_song(&mut a, at + 600), Some(5));
        assert_marker(&a, Some(600));
        assert_eq!(step_in_song(&mut a, at - 120), None);
        assert_marker(&a, None);
        assert_eq!(step_in_song(&mut a, at + 1920), None);
        assert_marker(&a, None);
    }
    assert_eq!(step_in_pattern(&mut a, id, 600), Some(5));
    assert_marker(&a, Some(600));
    assert_eq!(step_in_pattern(&mut a, other, 600), None);
    assert_marker(&a, None);
}

#[test]
fn preparing_marker_uses_the_pattern_position_including_a_song_repeat_start() {
    let (mut a, ctx, _, id) = drum_on_song();
    add_hits(&mut a, &ctx);
    let at = placed_at(&a, id);
    for start in [at - 120, at + 600, at + 1920] {
        transport::stop(&mut a);
        transport::repeat::start_song(&mut a, (start, start + 120));
        let marker = transport::preparing_row_for(&a, a.track);
        if start == at + 600 {
            let row = marker.unwrap();
            assert_eq!(crate::cursor::split::row_tick(&a, a.track, row), 600);
            assert!(row < a.loaded.as_ref().unwrap().display_rows[a.track].len());
        } else {
            assert_eq!(marker, None);
        }
    }
}

#[test]
fn two_restarts_keep_the_drum_body_song_selection_and_both_markers_together() {
    let (mut a, _, track, id) = drum_on_song();
    let at = placed_at(&a, id);
    crate::cursor::moves::at_tick(&mut a, 480);
    let music = a.loaded.as_ref().unwrap().music.clone();
    for _ in 0..2 {
        let saved = serde_json::to_vec(&a.session().unwrap()).unwrap();
        let session = serde_json::from_slice(&saved).unwrap();
        a = crate::view::tests::app(music.clone());
        a.restore_position(&session);
        assert_eq!(mode::editing(&a, track).unwrap().pattern, id);
        assert_eq!(
            a.patterns.workspace.song_cell.choice,
            Some(PatternOrScene::Pattern(id))
        );
        assert_eq!(crate::play::playback::location(&a).0, 480);
        assert!(crate::view::drums::is_open(&a));
        assert_eq!(step_in_song(&mut a, at + 600), Some(5));
        assert_marker(&a, Some(600));
        transport::stop(&mut a);
    }
}
