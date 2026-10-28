use super::*;
use crate::{
    edit,
    model::Track,
    score::{
        note_view::note_on,
        scene::arrangement::{self as grid, PatternOrScene},
    },
};
/// Place each `(measure, pattern)` on a new Song lane, with the CC 11 baseline the
/// fixture body needs to sound.
fn place_in_song(m: &mut Music, placed: &[(u64, u64)]) {
    let container = Catalog::read(m).unwrap().patterns[&placed[0].1].container;
    automation::set_midi_baseline(m, container, 4, Some(11), 127).unwrap();
    let lane = grid::add_lane(m, None).unwrap();
    for &(measure, id) in placed {
        grid::place(m, None, lane, measure, Some(PatternOrScene::Pattern(id))).unwrap();
    }
}
fn fixture() -> (Music, usize, u64) {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }, Track { events: vec![] }],
    };
    let source = operations::create_source(&mut m).unwrap();
    let id = operations::create(&mut m, source, None).unwrap();
    for (tick, status, data) in [
        (120, 0x94, vec![67, 83]),
        (120, 0xb4, vec![11, 72]),
        (1920, 0x84, vec![67, 95]),
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
    (m, source, id)
}
#[test]
fn bodies_duplicate_losslessly_with_boundary_gap_and_shared_container() {
    let (mut m, t, a) = fixture();
    let b = operations::create(&mut m, t, Some(a)).unwrap();
    let c = Catalog::read(&m).unwrap();
    let pa = &c.patterns[&a];
    let pb = &c.patterns[&b];
    assert_eq!(pa.container, pb.container);
    assert_eq!(pb.tick, 1921);
    let normalize = |p: &Definition| {
        body::extract(&m.tracks[t].events, p.range().unwrap())
            .unwrap()
            .into_iter()
            .map(|e| (e.tick - p.tick, e.status, e.data))
            .collect::<Vec<_>>()
    };
    assert_eq!(normalize(pa), normalize(pb));
    assert_eq!(normalize(pa).last().unwrap().2, vec![67, 95]);
    place_in_song(&mut m, &[(2, a), (3, b)]);
    crate::score::scene::sizing::song(&mut m, 3).unwrap();
    let bytes = crate::smf_io::encode(&m).unwrap();
    let parsed = crate::ffi::parse(&bytes).unwrap();
    let c = Catalog::read(&parsed).unwrap();
    assert_eq!(c.patterns[&b].tick, 1921);
    let song = crate::score::performance::song(&parsed).unwrap();
    let events: Vec<_> = song.tracks.iter().flat_map(|t| &t.events).collect();
    assert_eq!(events.iter().filter(|e| note_on(e)).count(), 2);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.status == 0x84)
            .map(|e| e.data[1])
            .collect::<Vec<_>>(),
        vec![95, 95]
    );
}
#[test]
fn explicit_empty_multimeasure_length_survives_storage_meter_and_track_reorder() {
    let (mut m, t, id) = fixture();
    operations::resize(
        &mut m,
        id,
        vec![
            Meter {
                numerator: 4,
                exponent: 2,
                ticks: 1920
            };
            3
        ],
    )
    .unwrap();
    let empty = operations::create(&mut m, t, None).unwrap();
    let container = Catalog::read(&m).unwrap().patterns[&id].container;
    operations::relocate(&mut m, id, container, 14000).unwrap();
    // Changing the storage track meter does not change either pattern's meter.
    edit::insert(
        &mut m.tracks[t],
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![7, 3, 24, 8],
        },
    );
    m.tracks.swap(t, 1);
    let bytes = crate::smf_io::encode(&m).unwrap();
    let parsed = crate::ffi::parse(&bytes).unwrap();
    let c = Catalog::read(&parsed).unwrap();
    assert_eq!(c.patterns[&id].length().unwrap(), 5760);
    assert_eq!(c.patterns[&id].track, 1);
    assert_eq!(c.patterns[&id].tick, 14000);
    assert!(body::extract(
        &parsed.tracks[1].events,
        c.patterns[&empty].range().unwrap()
    )
    .unwrap()
    .is_empty());
    assert_eq!(c.patterns[&empty].length().unwrap(), 1920);
}
#[test]
fn failed_storage_and_placement_edits_are_atomic_and_unknown_meta_survives() {
    let (mut m, t, id) = fixture();
    let lane = grid::add_lane(&mut m, None).unwrap();
    let unknown = Event {
        tick: 17,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: b"unknown owner".to_vec(),
    };
    edit::insert(&mut m.tracks[t], unknown.clone());
    let b = operations::create(&mut m, t, None).unwrap();
    let c = Catalog::read(&m).unwrap();
    let container = c.patterns[&id].container;
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(operations::relocate(&mut m, b, container, 1920).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(operations::resize(
        &mut m,
        id,
        vec![Meter {
            numerator: 3,
            exponent: 2,
            ticks: 1440
        }]
    )
    .is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(grid::place(
        &mut m,
        None,
        lane,
        2,
        Some(PatternOrScene::Pattern(u64::MAX))
    )
    .is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    operations::delete(&mut m, b).unwrap();
    assert!(m.tracks[t].events.iter().any(|e| e.data == unknown.data));
}
#[test]
fn corrupt_ids_missing_references_and_crossing_notes_reject() {
    let (mut m, t, id) = fixture();
    place_in_song(&mut m, &[(2, id)]);
    let mut bad = m.clone();
    bad.tracks.remove(t);
    assert!(Catalog::read(&bad).is_err());
    let meta = m.tracks[t]
        .events
        .iter()
        .find(|e| matches!(record(e), Ok(Some(Record::Definition { .. }))))
        .unwrap()
        .clone();
    let mut bad = m.clone();
    bad.tracks[t].events.push(meta);
    assert!(Catalog::read(&bad).is_err());
    let mut bad = m.clone();
    bad.tracks[t]
        .events
        .iter_mut()
        .find(|e| e.status == 0x84)
        .unwrap()
        .tick += 1;
    assert!(Catalog::read(&bad).is_err());
    let mut bad = m;
    bad.tracks[t].events.push(Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: PREFIX.to_vec(),
    });
    assert!(Catalog::read(&bad).is_err());
}

#[test]
fn extending_a_body_repacks_storage_without_changing_neighbor_events_or_musical_time() {
    let (mut m, t, a) = fixture();
    let b = operations::create(&mut m, t, Some(a)).unwrap();
    let before = Catalog::read(&m).unwrap();
    let pb = before.patterns[&b].clone();
    let neighbor = body::extract(&m.tracks[t].events, pb.range().unwrap())
        .unwrap()
        .iter()
        .map(|e| (e.tick, e.status, e.data.clone()))
        .collect::<Vec<_>>();
    operations::resize(
        &mut m,
        a,
        vec![
            Meter {
                numerator: 4,
                exponent: 2,
                ticks: 1920
            };
            3
        ],
    )
    .unwrap();
    let c = Catalog::read(&m).unwrap();
    assert!(c.patterns[&a].tick > pb.range().unwrap().1);
    assert_eq!(c.patterns[&a].length().unwrap(), 5760);
    assert_eq!(
        body::extract(&m.tracks[t].events, pb.range().unwrap())
            .unwrap()
            .iter()
            .map(|e| (e.tick, e.status, e.data.clone()))
            .collect::<Vec<_>>(),
        neighbor
    );
    let events = body::extract(&m.tracks[t].events, c.patterns[&a].range().unwrap()).unwrap();
    assert_eq!(events[0].tick - c.patterns[&a].tick, 120);
    assert_eq!(events.last().unwrap().tick - c.patterns[&a].tick, 1920);
}
