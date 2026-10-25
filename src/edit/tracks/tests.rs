use super::*;

#[test]
fn rename_preserves_timed_events_and_round_trips() {
    let mut music = new_song();
    let before = crate::smf_io::encode(&music).unwrap();
    rename(&mut music.tracks[0], "日本語 Piano").unwrap();
    let encoded = crate::smf_io::encode(&music).unwrap();
    let decoded = crate::ffi::parse(&encoded).unwrap();
    assert_eq!(decoded.tracks[0].name().as_deref(), Some("日本語 Piano"));
    music.tracks[0].events.remove(0);
    assert_eq!(crate::smf_io::encode(&music).unwrap(), before);
    let track = &mut music.tracks[0];
    track.events.insert(1, event(0, 0xff, 0x03, b"old"));
    track.events.insert(2, event(0, 0xff, 0x03, b"later"));
    let count = track.events.len();
    rename(track, "new").unwrap();
    assert_eq!(track.name().as_deref(), Some("new"));
    assert_eq!(track.events.len(), count);
    assert_eq!(track.events[2].data, b"later");
    rename(track, "").unwrap();
    assert_eq!(track.name().as_deref(), Some(""));
}

#[test]
fn normal_track_does_not_inherit_drum_routing_or_its_row_instruments() {
    let mut source = Track { events: vec![] };
    let settings = crate::score::drums::Settings::default();
    crate::score::drums::write(&mut source, Some(&settings)).unwrap();
    crate::instrument_layers::write(
        &mut source,
        &crate::instrument_layers::Settings {
            layers: (1..=4)
                .map(|id| crate::instrument_layers::Layer {
                    id,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        },
        &mut crate::patch_roots::Table::default(),
    )
    .unwrap();
    crate::score::drums::toggle(&mut source, 480, 0, 0).unwrap();
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![source.clone(), source],
    };
    add(&mut music).unwrap();
    assert!(crate::score::drums::read(&music.tracks[2])
        .unwrap()
        .is_none());
    assert_eq!(
        crate::instrument_layers::parse(&music.tracks[2])
            .unwrap()
            .layers
            .len(),
        1
    );
    assert!(music.tracks[2]
        .events
        .iter()
        .any(|e| note_on(e) && e.data[0] == 60));
}

#[test]
fn new_song_is_minimal_and_roundtrips() {
    let music = new_song();
    assert_eq!(
        (music.format, music.division, music.tracks.len()),
        (1, 480, 1)
    );
    let events = &music.tracks[0].events;
    assert_eq!(events.len(), 5);
    assert_eq!(events[2].data, [60, 100]);
    assert_eq!((events[3].tick, events[3].delta), (480, 480));
    let bytes = crate::smf_io::encode(&music).unwrap();
    assert_eq!(
        bytes,
        crate::smf_io::encode(&crate::ffi::parse(&bytes).unwrap()).unwrap()
    );
}

#[test]
fn only_common_head_events_of_two_or_more_note_tracks_are_inherited() {
    let mut music = new_song();
    add(&mut music).unwrap();
    assert_eq!(
        music.tracks[1].events.len(),
        3,
        "one source cannot initialize"
    );
    let common = event(0, 0xb2, 0, &[7, 90]);
    music.tracks[0].events.insert(0, common.clone());
    music.tracks[0].events.insert(0, common.clone());
    music.tracks[1].events.insert(0, common.clone());
    music.tracks[1].events.insert(1, event(0, 0xc2, 0, &[42]));
    music.tracks[1]
        .events
        .insert(2, event(0, 0x82, 0, &[60, 0]));
    // Identical late events and non-note tracks must not participate.
    for t in &mut music.tracks {
        t.events.push(event(960, 0xc2, 0, &[99]));
    }
    music.tracks.push(Track {
        events: vec![event(0, 0xff, 1, b"Conductor")],
    });
    add(&mut music).unwrap();
    let added = music.tracks.last().unwrap();
    assert_eq!(added.events.len(), 4);
    assert!(same(&added.events[0], &common));
    assert_eq!(added.events[1].status, 0x92);
    assert_eq!(added.events[1].data, [60, 100]);
    assert_eq!(added.events[2].tick, 480);
    // A third note track without the CC removes it from the intersection.
    add(&mut music).unwrap();
    assert!(same(&music.tracks.last().unwrap().events[0], &common));
    music.tracks[1].events.retain(|e| !same(e, &common));
    add(&mut music).unwrap();
    assert_eq!(music.tracks.last().unwrap().events.len(), 3);
}

#[test]
fn source_head_ticks_are_compared_independently_and_rebased() {
    let mut music = new_song();
    let mut other = music.tracks[0].clone();
    for e in &mut other.events {
        e.tick += 100;
    }
    music.tracks.push(other);
    add(&mut music).unwrap();
    assert_eq!(music.tracks[2].events.len(), 5);
    assert!(music.tracks[2].events[..3].iter().all(|e| e.tick == 0));
}

#[test]
fn formats_and_quarter_note_resolution() {
    for (format, division, duration) in [(0, 960, 960), (2, 96, 96), (1, 0xe728, 500)] {
        let mut music = new_song();
        music.format = format;
        music.division = division;
        add(&mut music).unwrap();
        assert_eq!(music.format, if format == 0 { 1 } else { format });
        assert_eq!(music.tracks[1].events[1].tick, duration);
    }
}
