use super::*;
use crate::{
    edit,
    score::scene::{
        self,
        arrangement::{self as grid, PatternOrScene},
    },
};
fn fixture() -> (Music, usize, u64, u64) {
    let mut m = edit::tracks::new_song();
    let t = operations::create_source(&mut m).unwrap();
    let a = operations::create(&mut m, t, None).unwrap();
    let b = operations::create(&mut m, t, None).unwrap();
    (m, t, a, b)
}
fn control(m: &mut Music, id: u64, cc: u8, value: u8) {
    let c = Catalog::read(m).unwrap();
    let p = &c.patterns[&id];
    edit::insert(
        &mut m.tracks[p.track],
        Event {
            tick: p.tick + 100,
            delta: 0,
            status: 0xb2,
            meta_type: 0,
            data: vec![cc, value],
        },
    );
    edit::refresh_deltas(&mut m.tracks[p.track]).unwrap();
}
#[test]
fn whole_occurrence_conflicts_include_scene_instances_but_distinct_targets_and_notes_can_merge() {
    let (mut m, t, a, b) = fixture();
    let owner = Catalog::read(&m).unwrap().patterns[&a].container;
    automation::set_midi_baseline(&mut m, owner, 2, Some(11), 95).unwrap();
    automation::set_midi_baseline(&mut m, owner, 2, Some(74), 64).unwrap();
    control(&mut m, a, 11, 15);
    control(&mut m, b, 74, 100);
    let s = grid::create_scene(&mut m, "A", 2, 2).unwrap();
    let lanes = scene::read(&m).unwrap().scenes[0].lanes.clone();
    grid::place(
        &mut m,
        Some(s),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(a)),
    )
    .unwrap();
    grid::place(
        &mut m,
        Some(s),
        lanes[1],
        1,
        Some(PatternOrScene::Pattern(b)),
    )
    .unwrap();
    let c = Catalog::read(&m).unwrap();
    let plan = render::expand(&m, &c, &[(a, 0), (b, 0)], 3840).unwrap();
    assert_eq!(plan.events.len(), 1);
    assert!(plan.events.contains_key(&t));
    let song = grid::add_lane(&mut m, None).unwrap();
    let song2 = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, song, 1, Some(PatternOrScene::Scene(s))).unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(
        grid::place(&mut m, None, song2, 1, Some(PatternOrScene::Scene(s)))
            .unwrap_err()
            .contains("Overlapping pattern control")
    );
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    // A second body with the same controller conflicts for its entire duration, even equal values.
    let mut next = m.clone();
    control(&mut next, b, 11, 15);
    assert!(Catalog::read(&next).is_err());
}
#[test]
fn midi_intervals_restore_baselines_without_restoring_between_adjacent_occurrences() {
    let (mut m, t, a, _) = fixture();
    let owner = Catalog::read(&m).unwrap().patterns[&a].container;
    automation::set_midi_baseline(&mut m, owner, 2, Some(11), 95).unwrap();
    control(&mut m, a, 11, 15);
    let c = Catalog::read(&m).unwrap();
    let plan = render::expand(&m, &c, &[(a, 0), (a, 1920)], 3840).unwrap();
    let events = &plan.events[&t];
    assert_eq!(
        events
            .iter()
            .map(|e| (e.tick, e.data[1]))
            .collect::<Vec<_>>(),
        vec![(0, 95), (100, 15), (1920, 95), (2020, 15), (3840, 95)]
    );
    // At the adjacent start only the new baseline is sent, not a duplicate restoration.
    assert_eq!(events.iter().filter(|e| e.tick == 1920).count(), 1);
}
#[test]
fn unsupported_cc_and_program_change_refuse_performance_without_deleting_content() {
    let (mut m, t, a, _) = fixture();
    control(&mut m, a, 101, 0);
    let before = crate::smf_io::encode(&m).unwrap();
    let c = Catalog::read(&m).unwrap();
    assert!(render::expand(&m, &c, &[(a, 0)], 1920)
        .err()
        .unwrap()
        .contains("unsupported"));
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    m.tracks[t]
        .events
        .iter_mut()
        .find(|e| e.status == 0xb2)
        .unwrap()
        .status = 0xc2;
    let c = Catalog::read(&m).unwrap();
    assert!(render::expand(&m, &c, &[(a, 0)], 1920).is_err());
    let index = m.tracks[t]
        .events
        .iter()
        .position(|e| e.status == 0xc2)
        .unwrap();
    for status in [0xa2, 0xd2, 0xf0, 0xf7] {
        m.tracks[t].events[index].status = status;
        m.tracks[t].events[index].data = if status == 0xd2 { vec![77] } else { vec![1, 2] };
        let before = crate::smf_io::encode(&m).unwrap();
        let c = Catalog::read(&m).unwrap();
        let error = render::expand(&m, &c, &[(a, 0)], 1920).err().unwrap();
        assert!(
            error.contains("unsupported") && error.contains("retained"),
            "{error}"
        );
        assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    }
}

#[test]
fn parameter_membership_moves_retarget_atomically_and_use_the_destination_baseline() {
    use crate::edit::parameter_curve::{Curve, Point, Target, ValueSpace};
    let (mut m, _, a, _) = fixture();
    let btrack = operations::create_source(&mut m).unwrap();
    let b = operations::create(&mut m, btrack, None).unwrap();
    let c = Catalog::read(&m).unwrap();
    let aowner = c.patterns[&a].container;
    let bowner = c.patterns[&b].container;
    let curve = |owner| Curve {
        version: 1,
        target: Target {
            group: owner,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "test".into(),
            parameter_id: "1".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: 0.0,
        maximum: 1.0,
        name: "Filter".into(),
        path: String::new(),
        points: vec![Point {
            tick: 0,
            value: 0.5,
        }],
        controls: vec![],
    };
    automation::write(&mut m, a, curve(aowner), Some(0.4)).unwrap();
    automation::write(&mut m, b, curve(bowner), Some(0.8)).unwrap();
    let scene = grid::create_scene(&mut m, "Control", 1, 2).unwrap();
    let lanes = scene::read(&m).unwrap().scenes[0].lanes.clone();
    grid::place(
        &mut m,
        Some(scene),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(a)),
    )
    .unwrap();
    grid::place(
        &mut m,
        Some(scene),
        lanes[1],
        1,
        Some(PatternOrScene::Pattern(b)),
    )
    .unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(operations::relocate(&mut m, a, bowner, 10000).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    grid::place(&mut m, Some(scene), lanes[1], 1, None).unwrap();
    operations::relocate(&mut m, a, bowner, 10000).unwrap();
    let c = Catalog::read(&m).unwrap();
    assert_eq!(c.patterns[&a].container, bowner);
    assert_eq!(
        automation::curves(&m, &c.patterns[&a]).unwrap()[0]
            .target
            .group,
        bowner
    );
    assert_eq!(
        render::expand(&m, &c, &[(a, 0)], 1920).unwrap().parameters[&btrack][0].baseline,
        0.8
    );
    assert_eq!(c.patterns[&b].tick, 0);
}
