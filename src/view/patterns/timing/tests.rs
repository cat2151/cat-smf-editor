use super::*;
use crate::{
    play::{playback::Mode, transport},
    view::patterns::{self, preview, workspace},
};
use cat_smf_editor::{
    edit,
    model::Event,
    score::{
        pattern::Catalog,
        scene::{self, arrangement::PatternOrScene},
    },
};

fn tempo(a: &mut App, tick: u64, micros: u32) {
    a.edit_tracks(|m| {
        edit::insert(
            &mut m.tracks[0],
            Event {
                tick,
                delta: 0,
                status: 0xff,
                meta_type: 0x51,
                data: vec![(micros >> 16) as u8, (micros >> 8) as u8, micros as u8],
            },
        );
        Ok(())
    })
    .unwrap();
}

fn fixture() -> (App, u64) {
    let (mut a, _, id) = patterns::edit_tests::fixture();
    tempo(&mut a, 0, 500_000);
    tempo(&mut a, 1920, 1_000_000);
    tempo(&mut a, 2400, 250_000);
    (a, id)
}

#[test]
fn candidate_uses_song_cursor_and_follows_changes_through_a_note_and_midi_events() {
    let (mut a, id) = fixture();
    a.patterns.workspace.song_cell.measure = 2;
    let bytes = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let p = preview::prepare(&a, id).unwrap();
    // Starting at Song tick 1920: 1 quarter at 60, then 3 quarters at 240.
    assert_eq!(p.active.end, 1.75);
    let n = &p.parts[0].2.notes[0];
    assert_eq!(n.start, 0.25);
    // The note spans local tick 120..600, crossing the change at local tick 480.
    assert_eq!(n.duration, 0.8125);
    assert!(p.parts[0].2.events.iter().any(|e| e.time == 1.0625));
    let t = &p.active.tempos[0];
    assert_eq!(t.tick_after(0, 1.0625), 600);
    assert_eq!(t.fractional_tick(1.0625), 600.0);
    assert_eq!(
        bytes,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
    a.patterns.workspace.song_cell.measure = 1;
    assert_eq!(preview::prepare(&a, id).unwrap().active.end, 2.0);
}

#[test]
fn each_song_placement_keeps_its_origin_and_candidates_leave_the_placement() {
    let (mut a, id) = fixture();
    let mut lane = 0;
    a.edit_tracks(|m| {
        lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 1, Some(PatternOrScene::Pattern(id)))?;
        arrangement::place(m, None, lane, 2, Some(PatternOrScene::Pattern(id)))
    })
    .unwrap();
    let c = eframe::egui::Context::default();
    for (measure, end) in [(1, 2.0), (2, 1.75)] {
        a.patterns.workspace.song_cell.measure = measure;
        workspace::edit(&mut a, &c, Pane::Song);
        // Moving the Song cursor after opening must not select another occurrence.
        a.patterns.workspace.song_cell.measure = 5;
        assert_eq!(preview::prepare(&a, id).unwrap().active.end, end);
    }
    // The binding follows placement ID when that placement moves.
    a.edit_tracks(|m| arrangement::move_part(m, None, (lane, 2), (lane, 3)))
        .unwrap();
    assert_eq!(pattern_tick(&a, id).unwrap(), 3840);
    patterns::library::edit(&mut a, id).unwrap();
    a.patterns.workspace.song_cell.measure = 1;
    assert_eq!(preview::prepare(&a, id).unwrap().active.end, 2.0);
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns
            .len(),
        2
    );
}

#[test]
fn pattern_inside_a_placed_scene_adds_both_offsets_and_scene_candidate_uses_cursor() {
    let (mut a, id) = fixture();
    let mut scene_id = 0;
    a.edit_tracks(|m| {
        scene_id = arrangement::create_scene(m, "A", 2, 1)?;
        let lane = scene::read(m)?.scenes[0].lanes[0];
        arrangement::place(
            m,
            Some(scene_id),
            lane,
            2,
            Some(PatternOrScene::Pattern(id)),
        )?;
        let lane = arrangement::add_lane(m, None)?;
        arrangement::place(m, None, lane, 3, Some(PatternOrScene::Scene(scene_id)))
    })
    .unwrap();
    tempo(&mut a, 5760, 1_000_000);
    a.patterns.workspace.song_cell.measure = 3;
    workspace::click_body(&mut a, Pane::Song);
    assert_eq!(scene_tick(&a, scene_id).unwrap(), 3840);
    a.patterns.workspace.scene_cell.measure = 2;
    workspace::click_body(&mut a, Pane::Scene);
    assert_eq!(pattern_tick(&a, id).unwrap(), 5760);
    assert_eq!(preview::prepare(&a, id).unwrap().active.end, 4.0);
    workspace::open_scene(&mut a, scene_id).unwrap();
    a.patterns.workspace.song_cell.measure = 1;
    assert_eq!(scene_tick(&a, scene_id).unwrap(), 0);
    // Opening a Scene candidate must not rebind the already opened Pattern occurrence.
    assert_eq!(pattern_tick(&a, id).unwrap(), 5760);
    let p = crate::play::arrangement::prepare(&a, Some(scene_id)).unwrap();
    assert_eq!(p.active.end, 3.75);
}

#[test]
fn seeking_and_repeating_keep_local_ticks_and_the_conductor_changes() {
    let (mut a, id) = fixture();
    patterns::library::edit(&mut a, id).unwrap();
    a.patterns.workspace.song_cell.measure = 2;
    preview::start(&mut a, id).unwrap();
    transport::advance(&mut a);
    a.patterns.workspace.song_cell.measure = 1;
    // A seek within the note must use the rebased time map, not conductor tick 480.
    let track = a.track;
    transport::timeline::seek(&mut a, track, 480);
    let p = a.playback.pending.as_ref().unwrap();
    assert_eq!(p.active.mode, Mode::Pattern(id));
    assert_eq!(p.active.start_seconds, 1.0);
    assert_eq!(p.active.end, 0.75);
    assert_eq!(p.parts[0].2.notes[0].duration, 0.0625);
    let p = transport::timeline::prepare(&a, 120, 600, &[track], Mode::Pattern(id), track).unwrap();
    assert_eq!(p.active.start_seconds, 0.25);
    assert_eq!(p.active.end, 0.8125);
}
