use super::*;
use cat_smf_editor::score::{
    pattern::operations,
    scene::{
        self,
        arrangement::{self as grid, PatternOrScene},
    },
};
#[test]
fn independent_multibar_playback_keeps_controls_and_rebuilds_seek_from_pattern_id() {
    use cat_smf_editor::{
        edit,
        model::Event,
        score::pattern::{automation, body},
    };
    let (mut a, source, id) = crate::view::patterns::edit_tests::fixture();
    let m = &mut a.loaded.as_mut().unwrap().music;
    operations::resize(m, id, body::parse_meters("4/4 4/4", 480).unwrap()).unwrap();
    let c = Catalog::read(m).unwrap();
    let p = c.patterns[&id].clone();
    automation::set_midi_baseline(m, p.container, 2, Some(64), 0).unwrap();
    for (tick, status, data) in [
        (p.tick, 0xb2, vec![64, 127]),
        (p.tick + 2400, 0x92, vec![60, 90]),
        (p.tick + 3000, 0x82, vec![60, 45]),
    ] {
        edit::insert(
            &mut m.tracks[source],
            Event {
                tick,
                delta: 0,
                status,
                meta_type: 0,
                data,
            },
        );
    }
    edit::refresh_deltas(&mut m.tracks[source]).unwrap();
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    let full = crate::play::transport::prepare(&a, Mode::Track).unwrap();
    assert_eq!(full.active.mode, Mode::Pattern(id));
    assert_eq!(full.active.end, 4.0);
    assert!(full.active.automation.owners[0].midi);
    assert_eq!(full.parts[0].2.notes.len(), 2);
    let seek = crate::play::selection_playback::skip(&a, full.active, 3.5).unwrap();
    assert_eq!(seek.active.start_seconds, 3.5);
    let words = seek.parts[0]
        .2
        .events
        .iter()
        .filter(|e| e.time == 3.5)
        .map(|e| e.word)
        .collect::<Vec<_>>();
    assert!(words.contains(&0x20b2_407f));
    assert!(words.contains(&0x2092_3c5a));
    assert!(words.contains(&0x2082_3c00));
    let mut a2 = a;
    crate::play::selection_playback::restore(&mut a2, seek);
    assert_eq!(
        a2.playback.pending.as_ref().unwrap().active.mode,
        Mode::Pattern(id)
    );
}
#[test]
fn shared_container_voice_membership_move_patch_changes_and_undo_affect_all_and_only_its_references(
) {
    let (mut a, source, p1) = crate::view::patterns::edit_tests::fixture();
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let p2 = *c.patterns.keys().find(|&&id| id != p1).unwrap();
    let mut ids = (0, 0, 0);
    a.edit_tracks(|m| {
        ids.0 = grid::create_scene(m, "A", 2, 2)?;
        let lanes = scene::read(m)?.scenes[0].lanes.clone();
        grid::place(
            m,
            Some(ids.0),
            lanes[0],
            1,
            Some(PatternOrScene::Pattern(p1)),
        )?;
        grid::place(
            m,
            Some(ids.0),
            lanes[1],
            1,
            Some(PatternOrScene::Pattern(p2)),
        )?;
        ids.1 = grid::add_lane(m, None)?;
        ids.2 = grid::add_lane(m, None)?;
        grid::place(m, None, ids.1, 1, Some(PatternOrScene::Scene(ids.0)))?;
        grid::place(m, None, ids.2, 3, Some(PatternOrScene::Scene(ids.0)))
    })
    .unwrap();
    let mut patch = a.loaded.as_ref().unwrap().patches[source].clone();
    patch.layers[0].settings.octave = 2;
    crate::player::selection::store_track(&mut a, source, &patch).unwrap();
    let p = prepare(&a, None).unwrap();
    assert_eq!(p.parts.len(), 1);
    assert_eq!(p.parts[0].2.notes.len(), 4);
    assert_eq!(p.parts[0].1, patch);
    let mut dest = 0;
    a.edit_tracks(|m| {
        dest = operations::create_source(m)?;
        Ok(())
    })
    .unwrap();
    let container = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == dest).then_some(id))
        .unwrap();
    crate::view::patterns::library::edit(&mut a, p1).unwrap();
    crate::view::patterns::membership::move_to(&mut a, p1, container).unwrap();
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, p1);
    assert_eq!(a.track, dest);
    let p = prepare(&a, None).unwrap();
    assert_eq!(p.parts.len(), 2);
    assert_eq!(
        p.parts
            .iter()
            .find(|p| p.0 == source)
            .unwrap()
            .2
            .notes
            .len(),
        2
    );
    assert_eq!(
        p.parts.iter().find(|p| p.0 == dest).unwrap().2.notes.len(),
        2
    );
    assert_eq!(
        crate::view::patterns::preview::sound(&a, p2).unwrap().1,
        patch
    );
    let placed = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    a.undo_music().unwrap();
    assert_eq!(a.track, source);
    assert_eq!(prepare(&a, None).unwrap().parts.len(), 1);
    assert_eq!(
        scene::read(&a.loaded.as_ref().unwrap().music).unwrap(),
        placed
    );
    // Lane moves preserve the container's sound, and never mix the original track notes.
    a.edit_tracks(|m| grid::move_part(m, None, (ids.2, 3), (ids.2, 5)))
        .unwrap();
    let p = prepare(&a, None).unwrap();
    assert_eq!(p.parts.len(), 1);
    assert_eq!(p.parts[0].1, patch);
    assert!(p.parts[0].2.notes.iter().all(|n| n.pitch == 67));
    assert!(crate::play::playback::parts(&a, Mode::AllTracks)
        .iter()
        .all(|p| p.0 != dest));
}
#[test]
fn pattern_scene_audition_tempo_is_explicit_and_independent_of_storage_or_song_tempo() {
    let (mut a, _, id) = crate::view::patterns::edit_tests::fixture();
    let mut scene_id = 0;
    a.edit_tracks(|m| {
        cat_smf_editor::edit::insert(
            &mut m.tracks[0],
            cat_smf_editor::model::Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x51,
                data: vec![15, 66, 64],
            },
        );
        scene_id = grid::create_scene(m, "B", 1, 1)?;
        let lane = scene::read(m)?.scenes[0].lanes[0];
        grid::place(
            m,
            Some(scene_id),
            lane,
            1,
            Some(PatternOrScene::Pattern(id)),
        )?;
        let lane = grid::add_lane(m, None)?;
        grid::place(m, None, lane, 1, Some(PatternOrScene::Scene(scene_id)))
    })
    .unwrap();
    assert_eq!(prepare(&a, None).unwrap().active.end, 4.0);
    assert_eq!(prepare(&a, Some(scene_id)).unwrap().active.end, 2.0);
    assert_eq!(
        crate::view::patterns::preview::prepare(&a, id)
            .unwrap()
            .active
            .end,
        2.0
    );
    a.patterns.tempo.0 = 90.0;
    assert!((prepare(&a, Some(scene_id)).unwrap().active.end - 8.0 / 3.0).abs() < 1e-9);
    assert_eq!(prepare(&a, None).unwrap().active.end, 4.0);
}
