//! Known Song positions, deliberately unrelated to where the pattern bodies are stored.
use super::*;
use crate::view::track_overview::{panes::Panes, song::Song};
use cat_smf_editor::score::{
    pattern::{self, operations},
    scene::{self, arrangement as grid},
};
use grid::PatternOrScene::{Pattern, Scene};
mod edges;
mod interaction;

pub(crate) fn fixture() -> (App, u64, u64, u64) {
    let (mut a, first) = crate::view::tests::unplaced::fixture();
    let mut ids = (0, 0);
    a.edit_tracks(|m| {
        let second = *pattern::Catalog::read(m)?
            .patterns
            .keys()
            .find(|&&id| id != first)
            .unwrap();
        ids.0 = second;
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 1, Some(Pattern(first)))?;
        grid::place(m, None, lane, 3, Some(Pattern(second)))?;
        let s = grid::create_scene(m, "Verse", 2, 1)?;
        ids.1 = s;
        let scene_lane = scene::read(m)?
            .scenes
            .iter()
            .find(|s| s.id == ids.1)
            .unwrap()
            .lanes[0];
        grid::place(m, Some(s), scene_lane, 2, Some(Pattern(first)))?;
        grid::place(m, None, lane, 4, Some(Scene(s)))?;
        let overlap = grid::add_lane(m, None)?;
        grid::place(m, None, overlap, 3, Some(Pattern(first)))?;
        let third = operations::create_source(m)?;
        let p = operations::create(m, third, None)?;
        let start = pattern::Catalog::read(m)?.patterns[&p].tick;
        for (tick, status) in [(start, 0x90), (start + 480, 0x80)] {
            cat_smf_editor::edit::insert(
                &mut m.tracks[third],
                event(tick, status, 0, vec![67, 100]),
            );
        }
        let other = grid::add_lane(m, None)?;
        grid::place(m, None, other, 2, Some(Pattern(p)))
    })
    .unwrap();
    a.patterns.workspace.song_cell.measure = 1;
    a.select_track(1);
    (a, first, ids.0, ids.1)
}

fn notes(layout: &crate::view::piano_roll::Layout) -> Vec<(u64, u8)> {
    super::pattern_edit::drawn(layout)
}

#[test]
fn song_draws_placements_and_scene_bodies_instead_of_storage_or_the_shown_edit() {
    let (mut a, first, _, _) = fixture();
    let expected = vec![(0, 60), (1920, 67), (3840, 60), (3840, 72), (7680, 60)];
    let p = Panes::read(&a, 0).unwrap();
    assert_eq!(notes(&p.overview), expected);
    assert_eq!(p.song.as_ref().unwrap().end, 9600);
    assert_eq!(p.tracks, vec![0, 1, 1, 2]);
    crate::view::patterns::library::edit(&mut a, first).unwrap();
    assert_eq!(notes(&Panes::read(&a, 0).unwrap().overview), expected);
    assert_eq!(
        Song::read(&a).unwrap().time.map(0).unwrap().signature(0),
        (4, 2)
    );
}

#[test]
fn unplaced_patterns_and_loop_are_excluded_even_when_their_storage_extends_past_song_end() {
    let (mut a, _, _, _) = fixture();
    a.edit_tracks(|m| {
        for _ in 0..10 {
            operations::create(m, 1, None)?;
        }
        operations::create_as(m, 1, pattern::Kind::Loop)?;
        Ok(())
    })
    .unwrap();
    let p = Panes::read(&a, 0).unwrap();
    assert_eq!(p.song.as_ref().unwrap().end, 9600);
    assert!((p.overview.size.x / p.overview.scale - 10080.0).abs() < 0.1);
    assert_eq!(
        p.song
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .map(|r| r.occurrences.len())
            .sum::<usize>(),
        5
    );
}

#[test]
fn disabled_scene_keeps_boundaries_and_duration_but_no_notes_or_controls() {
    let (mut a, _, _, id) = fixture();
    a.edit_tracks(|m| {
        let mut c = scene::read(m)?;
        c.song
            .iter_mut()
            .find(|p| p.part == Scene(id))
            .unwrap()
            .disabled = true;
        scene::write(m, &c)
    })
    .unwrap();
    let p = Panes::read(&a, 0).unwrap();
    assert_eq!(
        notes(&p.overview),
        vec![(0, 60), (1920, 67), (3840, 60), (3840, 72)]
    );
    let s = p.song.unwrap();
    assert_eq!(s.end, 9600);
    assert!(
        s.rows
            .iter()
            .flat_map(|r| &r.occurrences)
            .find(|o| o.start == 7680)
            .unwrap()
            .disabled
    );
}

#[test]
fn overview_keys_play_song_solo_and_jump_to_the_selected_overlapping_body() {
    let (a, first, second, _) = fixture();
    let (mut a, ctx) = opened_from(a);
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    press(&mut a, &ctx, &[egui::Key::L, egui::Key::Enter]);
    let p = a.playback.pending.as_ref().unwrap();
    assert_eq!(
        p.active.mode,
        crate::play::playback::Mode::Arrangement(None)
    );
    assert_eq!(p.active.start_seconds, 2.0);
    assert_eq!(p.active.end, 8.0);
    press(&mut a, &ctx, &[egui::Key::T]);
    assert_eq!(playing_tracks(&a), vec![1]);
    press(&mut a, &ctx, &[egui::Key::L, egui::Key::Space]);
    assert_eq!(playing_tracks(&a), vec![1]);
    assert_eq!(
        a.playback.pending.as_ref().unwrap().active.start_seconds,
        4.0
    );
    let song = Song::read(&a).unwrap();
    let target = song.at(a.track_overview.cell.0, 3840).unwrap().pattern;
    assert!([first, second].contains(&target));
    press(&mut a, &ctx, &[egui::Key::J]);
    let other = Song::read(&a)
        .unwrap()
        .at(a.track_overview.cell.0, 3840)
        .unwrap()
        .pattern;
    assert_ne!(target, other);
    press(&mut a, &ctx, &[egui::Key::G]);
    assert!(!a.track_overview.open);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, other);
    assert_eq!(crate::play::playback::location(&a).0, 0);
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, other).unwrap(),
        3840
    );
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        bytes
    );
}

#[test]
fn scene_jump_uses_the_right_song_occurrence_and_local_measure() {
    let (a, first, _, scene_id) = fixture();
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::L; 4]);
    assert_eq!(a.track_overview.cell.1, 7680);
    press(&mut a, &ctx, &[egui::Key::G]);
    let edit = a.patterns.editing.as_ref().unwrap();
    assert_eq!(edit.pattern, first);
    assert!(edit.placement.is_some());
    assert_eq!(crate::play::playback::location(&a).0, 0);
    assert_eq!(
        crate::view::patterns::timing::pattern_tick(&a, first).unwrap(),
        7680
    );
    assert!(scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .scenes
        .iter()
        .any(|s| s.id == scene_id));
}

#[test]
fn song_play_menu_owns_keys_and_escape_returns_without_closing_overview() {
    let (a, _, _, _) = fixture();
    let (mut a, ctx) = opened_from(a);
    press(&mut a, &ctx, &[egui::Key::P]);
    assert_eq!(commands::owner(&a, &ctx), Screen::Play);
    press(&mut a, &ctx, &[egui::Key::Escape]);
    assert_eq!(commands::owner(&a, &ctx), Screen::Overview);
    press(&mut a, &ctx, &[egui::Key::P, egui::Key::T]);
    assert_eq!(playing_tracks(&a), vec![1]);
    assert_eq!(commands::owner(&a, &ctx), Screen::Overview);
}

#[test]
fn seek_keeps_the_container_scope_and_never_moves_the_pattern_cursor() {
    let (mut a, first, _, _) = fixture();
    crate::view::patterns::library::edit(&mut a, first).unwrap();
    let before = crate::play::playback::location(&a).0;
    open(&mut a);
    play_track(&mut a, 1, 0);
    a.playback.repeat = Some((0, 9600));
    a.playback.repeat_mode = Some(crate::play::playback::Mode::Arrangement(None));
    crate::view::track_overview::song_playback::seek(&mut a, 3840);
    assert_eq!(playing_tracks(&a), vec![1]);
    assert_eq!(a.playback.repeat, Some((0, 9600)));
    assert_eq!(
        a.playback.pending.as_ref().unwrap().active.start_seconds,
        4.0
    );
    assert_eq!(crate::play::playback::location(&a).0, before);
}
