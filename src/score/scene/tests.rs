use super::*;
use crate::score::pattern::{self, operations as patterns};
use arrangement::{self as grid, PatternOrScene};
fn fixture() -> (Music, usize, u64) {
    let mut m = crate::edit::tracks::new_song();
    let t = patterns::create_source(&mut m).unwrap();
    let id = patterns::create(&mut m, t, None).unwrap();
    (m, t, id)
}
/// Every non-conductor track as saved, so arrangement edits must leave it byte-identical.
fn others(m: &Music) -> String {
    let saved = crate::ffi::parse(&crate::smf_io::encode(m).unwrap()).unwrap();
    format!("{:?}", &saved.tracks[1..])
}
#[test]
fn conductor_json_holds_lanes_and_measure_placements_without_new_tracks() {
    let (mut m, _, p) = fixture();
    let count = m.tracks.len();
    let tracks = others(&m);
    let s = grid::create_scene(&mut m, "A", 3, 2).unwrap();
    let c = read(&m).unwrap();
    let lanes = c.scenes[0].lanes.clone();
    grid::place(
        &mut m,
        Some(s),
        lanes[0],
        2,
        Some(PatternOrScene::Pattern(p)),
    )
    .unwrap();
    let song = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, song, 3, Some(PatternOrScene::Scene(s))).unwrap();
    assert_eq!(m.tracks.len(), count);
    assert_eq!(others(&m), tracks);
    let c = read(&m).unwrap();
    let placement = c.scenes[0].cells[0].id;
    let song_placement = c.song[0].id;
    let bytes = crate::smf_io::encode(&m).unwrap();
    let mut parsed = crate::ffi::parse(&bytes).unwrap();
    assert_eq!(read(&parsed).unwrap(), c);
    let meta = parsed.tracks[0].events.iter().find(|e| owned(e)).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&meta.data[PREFIX.len()..]).unwrap();
    assert_eq!(json["version"], 3);
    assert_eq!(json["scenes"][0]["cells"][0]["measure"], 1);
    assert_eq!(json["song"][0]["measure"], 3);
    assert!(json["song"][0].get("tick").is_none());
    assert!(json["scenes"][0]["cells"][0].get("tick").is_none());
    grid::move_part(&mut parsed, Some(s), (lanes[0], 2), (lanes[1], 1)).unwrap();
    assert_eq!(read(&parsed).unwrap().scenes[0].cells[0].id, placement);
    grid::move_part(&mut parsed, None, (song, 3), (song, 7)).unwrap();
    assert_eq!(read(&parsed).unwrap().song[0].id, song_placement);
    assert_eq!(
        grid::occurrences(&parsed, None).unwrap().0,
        vec![(p, 11520)]
    );
    let other = grid::create_scene(&mut parsed, "B", 1, 3).unwrap();
    grid::delete_scene(&mut parsed, other).unwrap();
    let c = read(&parsed).unwrap();
    let again = crate::ffi::parse(&crate::smf_io::encode(&parsed).unwrap()).unwrap();
    assert_eq!(read(&again).unwrap(), c);
    assert_eq!(c.lanes.len(), 3);
    assert_eq!(again.tracks.len(), count);
    assert_eq!(others(&again), tracks);
}
#[test]
fn non_v3_metadata_and_orphan_lanes_reject() {
    let (mut m, _, _) = fixture();
    grid::add_lane(&mut m, None).unwrap();
    let mut c = read(&m).unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    c.song_lanes.clear();
    assert!(write(&mut m, &c).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    let mut old = m.clone();
    let meta = old.tracks[0].events.iter_mut().find(|e| owned(e)).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&meta.data[PREFIX.len()..]).unwrap();
    json["version"] = 2.into();
    meta.data = [PREFIX, serde_json::to_vec(&json).unwrap().as_slice()].concat();
    assert!(read(&old).is_err());
}
#[test]
fn multimeasure_overlaps_meter_mismatch_and_length_edits_reject_atomically() {
    let (mut m, _, p) = fixture();
    patterns::resize(
        &mut m,
        p,
        vec![
            Meter {
                numerator: 4,
                exponent: 2,
                ticks: 1920
            };
            2
        ],
    )
    .unwrap();
    let s = grid::create_scene(&mut m, "B", 3, 2).unwrap();
    let lanes = read(&m).unwrap().scenes[0].lanes.clone();
    grid::place(
        &mut m,
        Some(s),
        lanes[0],
        1,
        Some(PatternOrScene::Pattern(p)),
    )
    .unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    for (lane, measure, part) in [
        (lanes[0], 2, PatternOrScene::Pattern(p)),
        (lanes[1], 3, PatternOrScene::Pattern(p)),
        (lanes[1], 1, PatternOrScene::Scene(s)),
        (u64::MAX, 1, PatternOrScene::Pattern(p)),
    ] {
        assert!(grid::place(&mut m, Some(s), lane, measure, Some(part)).is_err());
        assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    }
    assert!(patterns::resize(
        &mut m,
        p,
        vec![
            Meter {
                numerator: 3,
                exponent: 2,
                ticks: 1440
            };
            2
        ]
    )
    .is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(patterns::delete(&mut m, p).is_err());
}
#[test]
fn empty_scene_song_silence_and_invalid_lane_write_preserve_document() {
    let (mut m, _, _) = fixture();
    let s = grid::create_scene(&mut m, "Empty", 2, 1).unwrap();
    assert_eq!(grid::occurrences(&m, Some(s)).unwrap(), (vec![], 3840));
    let lane = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, lane, 2, Some(PatternOrScene::Scene(s))).unwrap();
    let id = read(&m).unwrap().song[0].id;
    grid::place(&mut m, None, lane, 2, None).unwrap();
    assert_eq!(grid::occurrences(&m, None).unwrap(), (vec![], 5760));
    let mut c = read(&m).unwrap();
    c.song.push(arrangement::Placement {
        id,
        lane: u64::MAX,
        measure: 1,
        part: PatternOrScene::Scene(s),
        disabled: false,
    });
    let before = crate::smf_io::encode(&m).unwrap();
    // Public write validates references as well as wire structure.
    assert!(write(&mut m, &c).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(pattern::Catalog::read(&m).is_ok());
}

#[test]
fn scene_meter_edits_keep_measure_positions_and_song_length_keeps_trailing_silence() {
    let (mut m, _, p) = fixture();
    let id = grid::create_scene(&mut m, "Meters", 2, 1).unwrap();
    let lane = read(&m).unwrap().scenes[0].lanes[0];
    grid::place(&mut m, Some(id), lane, 2, Some(PatternOrScene::Pattern(p))).unwrap();
    let placement = read(&m).unwrap().scenes[0].cells[0].id;
    let half = Meter {
        numerator: 2,
        exponent: 2,
        ticks: 960,
    };
    let whole = Meter {
        numerator: 4,
        exponent: 2,
        ticks: 1920,
    };
    sizing::scene(&mut m, id, vec![half.clone(), whole.clone(), half.clone()]).unwrap();
    let c = read(&m).unwrap();
    assert_eq!(c.scenes[0].cells[0].measure, 1);
    assert_eq!(c.scenes[0].cells[0].id, placement);
    assert_eq!(grid::occurrences(&m, Some(id)).unwrap().0, vec![(p, 960)]);
    let before = crate::smf_io::encode(&m).unwrap();
    // The measure number is the saved position, so the cell cannot drift onto another meter.
    assert!(sizing::scene(&mut m, id, vec![half.clone(), half, whole]).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(sizing::scene(
        &mut m,
        id,
        vec![
            Meter {
                numerator: 3,
                exponent: 2,
                ticks: 1440
            };
            3
        ]
    )
    .is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    let song = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, song, 2, Some(PatternOrScene::Pattern(p))).unwrap();
    sizing::song(&mut m, 4).unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(sizing::song(&mut m, 1).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    grid::place(&mut m, None, song, 2, None).unwrap();
    assert_eq!(grid::occurrences(&m, None).unwrap(), (vec![], 7680));
}
