use super::*;
use crate::score::{
    note_view::note_on,
    pattern::{automation, operations, Catalog},
    scene::arrangement::{self as grid, PatternOrScene},
};
fn fixture() -> (Music, usize, u64) {
    let mut m = edit::tracks::new_song();
    let owner = operations::create_source(&mut m).unwrap();
    let a = operations::create(&mut m, owner, None).unwrap();
    let b = operations::create(&mut m, owner, None).unwrap();
    let c = Catalog::read(&m).unwrap();
    for (id, pitch) in [(a, 60), (b, 67)] {
        let p = &c.patterns[&id];
        for (offset, status, velocity) in [(120, 0x92, 100), (360, 0x82, 37)] {
            edit::insert(
                &mut m.tracks[owner],
                Event {
                    tick: p.tick + offset,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![pitch, velocity],
                },
            );
        }
    }
    edit::refresh_deltas(&mut m.tracks[owner]).unwrap();
    let s = grid::create_scene(&mut m, "A", 2, 1).unwrap();
    let lane = scene::read(&m).unwrap().scenes[0].lanes[0];
    grid::place(&mut m, Some(s), lane, 1, Some(PatternOrScene::Pattern(a))).unwrap();
    grid::place(&mut m, Some(s), lane, 2, Some(PatternOrScene::Pattern(b))).unwrap();
    let lane = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, lane, 1, Some(PatternOrScene::Scene(s))).unwrap();
    grid::place(&mut m, None, lane, 3, Some(PatternOrScene::Scene(s))).unwrap();
    crate::score::scene::sizing::song(&mut m, 6).unwrap();
    (m, owner, a)
}
#[test]
fn song_export_is_flat_lossless_and_keeps_silence_settings_and_exact_parameter_windows() {
    use crate::edit::parameter_curve::{Curve, Point, Target, ValueSpace};
    let (mut m, owner, id) = fixture();
    let container = Catalog::read(&m).unwrap().patterns[&id].container;
    automation::set_midi_baseline(&mut m, container, 2, Some(11), 100).unwrap();
    edit::insert(
        &mut m.tracks[owner],
        Event {
            tick: 0,
            delta: 0,
            status: 0xb2,
            meta_type: 0,
            data: vec![11, 50],
        },
    );
    edit::refresh_deltas(&mut m.tracks[owner]).unwrap();
    automation::write(
        &mut m,
        id,
        Curve {
            version: 1,
            target: Target {
                group: container,
                layer: 1,
                plugin_format: "CLAP".into(),
                plugin_id: "fixture".into(),
                parameter_id: "101".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.,
            maximum: 1.,
            name: "Cutoff".into(),
            path: "Filter".into(),
            points: vec![Point {
                tick: 0,
                value: 0.7,
            }],
            controls: vec![],
        },
        Some(0.2),
    )
    .unwrap();
    let before = crate::smf_io::encode(&m).unwrap();
    let flat = song(&m).unwrap();
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert_eq!(flat.tracks.len(), 2);
    assert_eq!(
        flat.tracks
            .iter()
            .flat_map(|t| &t.events)
            .filter(|e| note_on(e))
            .count(),
        4
    );
    assert!(flat
        .tracks
        .iter()
        .flat_map(|t| &t.events)
        .all(|e| !arrangement_meta(e)));
    assert!(flat
        .tracks
        .iter()
        .all(|t| t.events.last().unwrap().tick == 11520));
    assert_eq!(
        flat.tracks[1]
            .events
            .iter()
            .filter(|e| e.status == 0x82)
            .map(|e| (e.tick, e.data[1]))
            .collect::<Vec<_>>(),
        vec![(360, 37), (2280, 37), (4200, 37), (6120, 37)]
    );
    let windows = read(&flat).unwrap();
    assert_eq!(
        windows[&1]
            .iter()
            .map(|w| (w.start, w.end, w.baseline))
            .collect::<Vec<_>>(),
        vec![(0, 1920, 0.2), (3840, 5760, 0.2)]
    );
    let restored = crate::ffi::parse(&crate::smf_io::encode(&flat).unwrap()).unwrap();
    assert_eq!(read(&restored).unwrap()[&1].len(), 2);
}
#[test]
fn regenerate_must_preserve_pattern_and_performance_drum_note_offs() {
    let (mut m, owner, id) = fixture();
    let mut settings = crate::score::drums::Settings::default();
    for (i, row) in settings.rows.iter_mut().enumerate() {
        row.layer = i as u64 + 1;
    }
    crate::score::drums::write(&mut m.tracks[owner], Some(&settings)).unwrap();
    let p = Catalog::read(&m).unwrap().patterns[&id].clone();
    for e in &mut m.tracks[owner].events {
        if e.status == 0x92 || e.status == 0x82 {
            e.data[0] = 36;
        }
    }
    let before = pattern::body::extract(&m.tracks[owner].events, p.range().unwrap()).unwrap();
    let signature = |events: Vec<Event>| {
        events
            .into_iter()
            .map(|e| (e.tick, e.status, e.data))
            .collect::<Vec<_>>()
    };
    edit::regenerate(&mut m).unwrap();
    assert_eq!(
        signature(pattern::body::extract(&m.tracks[owner].events, p.range().unwrap()).unwrap()),
        signature(before)
    );
    let mut flat = song(&m).unwrap();
    let before = signature(flat.tracks[1].events.clone());
    edit::regenerate(&mut flat).unwrap();
    assert_eq!(signature(flat.tracks[1].events.clone()), before);
}
#[test]
fn commented_out_placements_leave_no_notes_but_keep_the_length() {
    let (mut m, _, _) = fixture();
    let notes = |m: &Music| {
        let flat = song(m).unwrap();
        let end = flat.tracks[0].events.last().unwrap().tick;
        let pitches = flat
            .tracks
            .iter()
            .flat_map(|t| &t.events)
            .filter(|e| note_on(e))
            .map(|e| (e.tick, e.data[0]))
            .collect::<Vec<_>>();
        (pitches, end)
    };
    let c = scene::read(&m).unwrap();
    let (song_lane, scene_lane, s) = (c.song[0].lane, c.scenes[0].lanes[0], c.scenes[0].id);
    grid::toggle(&mut m, None, song_lane, 4).unwrap();
    assert_eq!(notes(&m), (vec![(120, 60), (2040, 67)], 11520));
    grid::toggle(&mut m, Some(s), scene_lane, 2).unwrap();
    assert_eq!(notes(&m), (vec![(120, 60)], 11520));
}
