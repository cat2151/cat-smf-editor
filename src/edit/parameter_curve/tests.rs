use super::*;
use crate::{
    host::PluginKey,
    model::{Event, Music, Track},
    score::{automation, layer_track::Patch},
};

fn event(tick: u64, status: u8, kind: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: kind,
        data: data.to_vec(),
    }
}
fn song() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                event(0, 0xff, 0x51, &[7, 0xa1, 0x20]),
                event(0, 0xff, 0x58, &[4, 2, 24, 8]),
                event(0, 0x90, 0, &[60, 100]),
                event(20, 0xf0, 0, &[1, 2, 0xf7]),
                event(30, 0xff, 0x7f, b"unknown"),
                event(480, 0x80, 0, &[60, 0]),
                event(960, 0xff, 0x2f, &[]),
            ],
        }],
    };
    crate::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
    music
}
fn curve() -> Curve {
    Curve {
        version: 1,
        target: Target {
            group: 0,
            layer: 1,
            plugin_format: "CLAP".into(),
            plugin_id: "fixture".into(),
            parameter_id: "101".into(),
        },
        value_space: ValueSpace::Plain,
        minimum: -10.0,
        maximum: 10.0,
        name: "Cutoff".into(),
        path: "Filter/1".into(),
        points: vec![Point {
            tick: 120,
            value: -5.0,
        }],
        controls: vec![],
    }
}
fn key() -> PluginKey {
    PluginKey {
        format: "CLAP".into(),
        id: "fixture".into(),
        name: "Fixture".into(),
        vendor: String::new(),
        bundle_path: String::new(),
    }
}

#[test]
fn outside_control_geometry_keeps_native_output_bounded_and_rejects_nonfinite_values() {
    let mut c = curve();
    c.target.group = 1;
    c.add(600, 8.0).unwrap();
    for value in [-100.0, 100.0] {
        c.move_control(0, 360, value).unwrap();
        c.validate().unwrap();
        assert_eq!(
            c.value_at(360.0),
            Some(if value < 0.0 { -10.0 } else { 10.0 })
        );
        assert_eq!(c.value_at(120.0), Some(-5.0));
        assert_eq!(c.value_at(600.0), Some(8.0));
    }
    let before = c.clone();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(c.move_control(0, 360, value).is_err());
        assert_eq!(c, before);
        c.controls[0].value = value;
        assert!(c.validate().is_err());
        c = before.clone();
    }
    assert!(c.move_point(0, 120, 11.0).is_err());
}
fn bytes(music: &Music) -> Vec<u8> {
    crate::smf_io::encode(music).unwrap()
}

#[test]
fn parameter_roundtrip_shape_and_unknown_events_are_preserved() {
    let mut music = song();
    let old = music.tracks[0].events.clone();
    let index = create(&mut music, 0, curve()).unwrap();
    assert_eq!(index, 1);
    let mut c = read(&music).unwrap().curves[&index].clone();
    c.add(600, 8.0).unwrap();
    c.move_control(0, 250, 9.0).unwrap();
    write(&mut music, index, &c).unwrap();
    let reparsed = crate::ffi::parse(&bytes(&music)).unwrap();
    assert_eq!(read(&reparsed).unwrap().curves[&index], c);
    assert_eq!(read(&reparsed).unwrap().owner(index), Some(0));
    for tick in [0.0, 120.0, 250.5, 599.5, 900.0] {
        assert_eq!(
            read(&reparsed).unwrap().curves[&index].value_at(tick),
            c.value_at(tick)
        );
    }
    let retained: Vec<_> = music.tracks[0]
        .events
        .iter()
        .filter(|e| !group_event(e))
        .map(|e| (e.tick, e.status, e.meta_type, &e.data))
        .collect();
    let expected: Vec<_> = old
        .iter()
        .map(|e| (e.tick, e.status, e.meta_type, &e.data))
        .collect();
    assert_eq!(retained, expected);
    assert!(!music.tracks[index]
        .events
        .iter()
        .any(|e| matches!(e.status & 0xf0, 0x80 | 0x90)));
    assert_eq!(music.tracks[index].events.last().unwrap().tick, 960);
    let before = bytes(&music);
    assert_eq!(create(&mut music, 0, curve()).unwrap(), index);
    assert_eq!(bytes(&music), before);
}

#[test]
fn empty_curve_retains_identity_reopens_and_does_not_shorten_end() {
    let mut music = song();
    let index = create(&mut music, 0, curve()).unwrap();
    let mut c = read(&music).unwrap().curves[&index].clone();
    c.add(2000, 2.0).unwrap();
    write(&mut music, index, &c).unwrap();
    c.remove(1).unwrap();
    c.remove(0).unwrap();
    write(&mut music, index, &c).unwrap();
    let c = &read(&music).unwrap().curves[&index];
    assert!(c.points.is_empty() && c.controls.is_empty());
    assert_eq!(c.value_at(10.0), None);
    assert_eq!(music.tracks[index].events.last().unwrap().tick, 2000);
    assert_eq!(read(&music).unwrap().owner(index), Some(0));
}

#[test]
fn parameter_edits_are_atomic_and_validate_order_range_space_and_schema() {
    let mut music = song();
    let index = create(&mut music, 0, curve()).unwrap();
    let good = read(&music).unwrap().curves[&index].clone();
    let before = bytes(&music);
    let mut invalid = Vec::new();
    let mut c = good.clone();
    c.points[0].value = f64::NAN;
    invalid.push(c);
    let mut c = good.clone();
    c.minimum = f64::NEG_INFINITY;
    invalid.push(c);
    let mut c = good.clone();
    c.points.push(c.points[0]);
    c.controls.push(c.points[0]);
    invalid.push(c);
    let mut c = good.clone();
    c.target.plugin_format = "VST3".into();
    invalid.push(c);
    let mut c = good.clone();
    c.version = 2;
    invalid.push(c);
    let mut c = good.clone();
    c.controls.push(c.points[0]);
    invalid.push(c);
    let mut c = good.clone();
    c.target.group = 0;
    invalid.push(c);
    let mut c = good.clone();
    c.points[0].tick = u64::MAX;
    invalid.push(c);
    for c in invalid {
        assert!(write(&mut music, index, &c).is_err());
        assert_eq!(bytes(&music), before);
    }
    let mut c = good.clone();
    c.add(300, 1.0).unwrap();
    c.controls[0].tick = 301;
    assert!(write(&mut music, index, &c).is_err());
    assert_eq!(bytes(&music), before);
    c = good;
    c.add(300, 1.0).unwrap();
    let old = c.clone();
    assert!(c.move_point(0, 300, 0.0).is_err());
    assert_eq!(c, old);
}

#[test]
fn corrupt_metadata_is_distinct_from_missing_owner_or_conflicting_target() {
    let mut music = song();
    let index = create(&mut music, 0, curve()).unwrap();
    let valid = music.clone();
    let meta = music.tracks[index]
        .events
        .iter()
        .find(|e| owned(e))
        .unwrap()
        .clone();
    music.tracks[index].events.insert(0, meta.clone());
    assert!(read(&music).is_err());
    music = valid.clone();
    music.tracks[index]
        .events
        .iter_mut()
        .find(|e| owned(e))
        .unwrap()
        .tick = 1;
    assert!(read(&music).is_err());
    music = valid.clone();
    let group = music.tracks[0]
        .events
        .iter()
        .find(|e| group_event(e))
        .unwrap()
        .clone();
    music.tracks.push(Track {
        events: vec![group, event(960, 0xff, 0x2f, &[])],
    });
    assert!(read(&music).is_err());
    music = valid.clone();
    music.tracks[0].events.retain(|e| !group_event(e));
    let missing = read(&music).unwrap();
    assert_eq!(missing.owner(index), None);
    assert!(automation::resolve(&missing, index, &[(0, Patch::default())], Some(&key())).is_err());
    music = valid.clone();
    music.tracks.push(valid.tracks[index].clone());
    let conflict = read(&music).unwrap();
    assert_eq!(conflict.conflicts(), vec![vec![1, 2]]);
    assert!(automation::resolve(&conflict, index, &[(0, Patch::default())], Some(&key())).is_err());
    let mut empty = conflict.curves[&2].clone();
    empty.remove(0).unwrap();
    write(&mut music, 2, &empty).unwrap();
    assert!(read(&music).unwrap().conflicts().is_empty());
}

#[test]
fn owner_identity_survives_track_addition_and_reordering() {
    let mut music = song();
    assert_eq!(create(&mut music, 0, curve()).unwrap(), 1);
    crate::edit::tracks::add(&mut music).unwrap();
    assert!(!music.tracks[2].events.iter().any(group_event));
    music.tracks.swap(1, 2);
    assert_eq!(read(&music).unwrap().owner(2), Some(0));
    let mut instrument = song().tracks.remove(0);
    instrument.events.retain(|e| !group_event(e));
    music.tracks.push(instrument);
    music.tracks.swap(0, 3); // Simulate reordering the owner; restore conductor-only tempo placement not needed here.
    assert_eq!(read(&music).unwrap().owner(2), Some(3));
}

#[test]
fn native_identity_and_value_contract_do_not_use_labels_or_list_indices() {
    let mut music = song();
    let index = create(&mut music, 0, curve()).unwrap();
    let collection = read(&music).unwrap();
    let patches = vec![(0, Patch::default())];
    assert_eq!(
        automation::resolve(&collection, index, &patches, Some(&key()))
            .unwrap()
            .layer_index,
        0
    );
    let mut changed = key();
    changed.id = "different".into();
    assert!(automation::resolve(&collection, index, &patches, Some(&changed)).is_err());
    let c = &collection.curves[&index];
    let spec = || automation::ParameterSpec {
        stable_id: "101",
        minimum: -10.0,
        maximum: 10.0,
        normalized: false,
        eligible: true,
    };
    automation::validate_parameter(c, spec()).unwrap();
    let mut p = spec();
    p.stable_id = "118";
    assert!(automation::validate_parameter(c, p).is_err());
    let mut p = spec();
    p.minimum = 0.0;
    assert!(automation::validate_parameter(c, p).is_err());
    let mut p = spec();
    p.eligible = false;
    assert!(automation::validate_parameter(c, p).is_err());
}

#[test]
fn new_meta_does_not_change_old_songs_and_rejects_other_formats() {
    let music = song();
    let before = bytes(&music);
    let mut next = music.clone();
    assert!(read(&next).unwrap().curves.is_empty());
    crate::edit::regenerate(&mut next).unwrap();
    assert_eq!(bytes(&next), before);
    for format in [0, 2] {
        let mut m = song();
        m.format = format;
        let before = bytes(&m);
        assert!(create(&mut m, 0, curve()).is_err());
        assert_eq!(bytes(&m), before);
    }
}

#[test]
fn deleted_owner_id_is_not_reused_by_another_owner() {
    let mut music = song();
    let first = create(&mut music, 0, curve()).unwrap();
    let old_group = read(&music).unwrap().curves[&first].target.group;
    music.tracks[0].events.retain(|e| !group_event(e));
    let second = create(&mut music, 0, curve()).unwrap();
    let collection = read(&music).unwrap();
    assert_ne!(collection.curves[&second].target.group, old_group);
    assert_eq!(collection.owner(first), None);
    assert_eq!(collection.owner(second), Some(0));
}
