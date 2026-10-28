use super::*;
use crate::score::pattern::operations as patterns;
fn fixture() -> (Music, u64, u64) {
    let mut m = crate::edit::tracks::new_song();
    let source = patterns::create_source(&mut m).unwrap();
    let a = patterns::create(&mut m, source, None).unwrap();
    let b = patterns::create(&mut m, source, None).unwrap();
    (m, a, b)
}
#[test]
fn lanes_roundtrip_shared_references_overlap_atomicity_and_silence() {
    let (mut m, a, b) = fixture();
    let count = m.tracks.len();
    let scene = create_scene(&mut m, "A", 3, 2).unwrap();
    let c = read(&m).unwrap();
    let lanes = c.scenes[0].lanes.clone();
    assert!(lanes
        .iter()
        .all(|id| !Catalog::read(&m).unwrap().tracks.contains_key(id)));
    place(
        &mut m,
        Some(scene),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(a)),
    )
    .unwrap();
    place(
        &mut m,
        Some(scene),
        lanes[1],
        2,
        Some(PatternOrScene::Pattern(b)),
    )
    .unwrap();
    let song = add_lane(&mut m, None).unwrap();
    let song2 = add_lane(&mut m, None).unwrap();
    place(&mut m, None, song, 1, Some(PatternOrScene::Scene(scene))).unwrap();
    place(&mut m, None, song, 4, Some(PatternOrScene::Pattern(b))).unwrap();
    place(&mut m, None, song2, 1, Some(PatternOrScene::Scene(scene))).unwrap();
    assert_eq!(m.tracks.len(), count);
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(place(&mut m, None, song, 2, Some(PatternOrScene::Pattern(a))).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(patterns::delete(&mut m, a).is_err());
    let parsed = crate::ffi::parse(&before).unwrap();
    assert_eq!(read(&m).unwrap(), read(&parsed).unwrap());
    let (notes, end) = occurrences(&parsed, None).unwrap();
    assert_eq!(notes.len(), 5);
    assert_eq!(end, 7680);
    move_part(&mut m, None, (song, 4), (song2, 5)).unwrap();
    assert_eq!(
        cell(&read(&m).unwrap(), None, song2, 5),
        Some(PatternOrScene::Pattern(b))
    );
    let (notes, end) = occurrences(&m, Some(scene)).unwrap();
    assert_eq!(notes, vec![(a, 0), (b, 1920)]);
    assert_eq!(end, 5760);
}
#[test]
fn empty_scene_and_invalid_scene_cells_are_atomic() {
    let (mut m, a, _) = fixture();
    let id = create_scene(&mut m, "Empty", 2, 1).unwrap();
    assert_eq!(occurrences(&m, Some(id)).unwrap(), (vec![], 3840));
    let lane = read(&m).unwrap().scenes[0].lanes[0];
    let before = crate::smf_io::encode(&m).unwrap();
    for (lane, measure, part) in [
        (lane, 3, PatternOrScene::Pattern(a)),
        (lane, 1, PatternOrScene::Scene(id)),
        (u64::MAX, 1, PatternOrScene::Pattern(a)),
    ] {
        assert!(place(&mut m, Some(id), lane, measure, Some(part)).is_err());
        assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    }
}
fn conductor_json(m: &Music) -> Vec<u8> {
    m.tracks[0]
        .events
        .iter()
        .find(|e| super::super::owned(e))
        .unwrap()
        .data
        .clone()
}
#[test]
fn commented_out_song_and_scene_placements_are_silent_but_keep_length_and_cells() {
    let (mut m, a, b) = fixture();
    let s = create_scene(&mut m, "A", 2, 2).unwrap();
    let lanes = read(&m).unwrap().scenes[0].lanes.clone();
    place(
        &mut m,
        Some(s),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(a)),
    )
    .unwrap();
    place(
        &mut m,
        Some(s),
        lanes[1],
        1,
        Some(PatternOrScene::Pattern(b)),
    )
    .unwrap();
    let one = add_lane(&mut m, None).unwrap();
    let two = add_lane(&mut m, None).unwrap();
    place(&mut m, None, one, 1, Some(PatternOrScene::Pattern(a))).unwrap();
    place(&mut m, None, two, 1, Some(PatternOrScene::Pattern(b))).unwrap();
    place(&mut m, None, one, 3, Some(PatternOrScene::Scene(s))).unwrap();
    // Placement order in the JSON follows edit history until the next read-and-write.
    let mut normalized = m.clone();
    write(&mut normalized, &read(&m).unwrap()).unwrap();
    let enabled = conductor_json(&normalized);
    assert!(!String::from_utf8_lossy(&enabled).contains("disabled"));
    let (all, end) = occurrences(&m, None).unwrap();
    let ordered = |mut v: Vec<(u64, u64)>| {
        v.sort_by_key(|&(id, tick)| (tick, id));
        v
    };
    assert_eq!(all, ordered(vec![(a, 0), (b, 0), (a, 3840), (b, 3840)]));
    assert_eq!(end, 7680);

    toggle(&mut m, None, two, 1).unwrap();
    assert_eq!(
        occurrences(&m, None).unwrap(),
        (ordered(vec![(a, 0), (a, 3840), (b, 3840)]), end)
    );
    // A measure inside a multimeasure Scene block selects the whole block.
    toggle(&mut m, None, one, 4).unwrap();
    assert_eq!(occurrences(&m, None).unwrap(), (vec![(a, 0)], end));
    assert!(place(&mut m, None, one, 4, Some(PatternOrScene::Pattern(b))).is_err());
    toggle(&mut m, None, one, 3).unwrap();
    toggle(&mut m, Some(s), lanes[1], 1).unwrap();
    assert_eq!(occurrences(&m, Some(s)).unwrap(), (vec![(a, 0)], 3840));
    assert_eq!(
        occurrences(&m, None).unwrap(),
        (vec![(a, 0), (a, 3840)], end)
    );
    assert!(toggle(&mut m, Some(s), lanes[1], 2).is_err());
    assert!(toggle(&mut m, None, two, 3).is_err());

    let parsed = crate::ffi::parse(&crate::smf_io::encode(&m).unwrap()).unwrap();
    let c = read(&parsed).unwrap();
    assert_eq!(c, read(&m).unwrap());
    assert!(c.scenes[0].cells.iter().any(|p| p.disabled));
    assert!(c.song.iter().any(|p| p.disabled));
    let describe = super::super::describe(&conductor_json(&parsed));
    assert!(describe.contains("1 commented out]"), "{describe}");
    assert!(describe.ends_with("ticks, 1 commented out"), "{describe}");

    toggle(&mut m, None, two, 1).unwrap();
    toggle(&mut m, Some(s), lanes[1], 1).unwrap();
    assert_eq!(conductor_json(&m), enabled);
}
#[test]
fn moving_keeps_comment_out_and_replacing_restores_it() {
    let (mut m, a, b) = fixture();
    let s = create_scene(&mut m, "A", 3, 2).unwrap();
    let lanes = read(&m).unwrap().scenes[0].lanes.clone();
    place(
        &mut m,
        Some(s),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(a)),
    )
    .unwrap();
    toggle(&mut m, Some(s), lanes[0], 1).unwrap();
    move_part(&mut m, Some(s), (lanes[0], 1), (lanes[1], 3)).unwrap();
    assert!(read(&m).unwrap().scenes[0].cells[0].disabled);
    assert_eq!(occurrences(&m, Some(s)).unwrap().0, vec![]);
    let song = add_lane(&mut m, None).unwrap();
    place(&mut m, None, song, 2, Some(PatternOrScene::Pattern(a))).unwrap();
    toggle(&mut m, None, song, 2).unwrap();
    move_part(&mut m, None, (song, 2), (song, 5)).unwrap();
    assert!(read(&m).unwrap().song[0].disabled);
    place(&mut m, None, song, 5, Some(PatternOrScene::Pattern(b))).unwrap();
    assert!(!read(&m).unwrap().song[0].disabled);
    assert_eq!(occurrences(&m, None).unwrap().0, vec![(b, 7680)]);
}
#[test]
fn restoring_into_a_control_conflict_rejects_and_keeps_the_song() {
    let (mut m, a, b) = fixture();
    let catalog = Catalog::read(&m).unwrap();
    let owner = catalog.patterns[&a].container;
    crate::score::pattern::automation::set_midi_baseline(&mut m, owner, 2, Some(11), 95).unwrap();
    for id in [a, b] {
        let p = Catalog::read(&m).unwrap().patterns[&id].clone();
        crate::edit::insert(
            &mut m.tracks[p.track],
            crate::model::Event {
                tick: p.tick + 100,
                delta: 0,
                status: 0xb2,
                meta_type: 0,
                data: vec![11, 15],
            },
        );
        crate::edit::refresh_deltas(&mut m.tracks[p.track]).unwrap();
    }
    let one = add_lane(&mut m, None).unwrap();
    let two = add_lane(&mut m, None).unwrap();
    place(&mut m, None, one, 1, Some(PatternOrScene::Pattern(a))).unwrap();
    assert!(
        place(&mut m, None, two, 1, Some(PatternOrScene::Pattern(b)))
            .unwrap_err()
            .contains("Overlapping pattern control")
    );
    toggle(&mut m, None, one, 1).unwrap();
    place(&mut m, None, two, 1, Some(PatternOrScene::Pattern(b))).unwrap();
    assert_eq!(occurrences(&m, None).unwrap().0, vec![(b, 0)]);
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(toggle(&mut m, None, one, 1)
        .unwrap_err()
        .contains("Overlapping pattern control"));
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
}
