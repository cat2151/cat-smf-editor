use cat_smf_editor::{
    model::{Event, Music, Track},
    score::key_signature::{KeySignature, KeySignatures},
};

fn signature(tick: u64, sharps: i8, mode: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![sharps as u8, mode],
    }
}

#[test]
fn d_minor_fifths_above_tonic_are_octave_independent() {
    let key = KeySignature::from_event(&signature(0, -1, 1)).unwrap();
    assert_eq!(key.name(), "D minor");
    assert_eq!(key.tonic(), 2);
    assert!(key.minor());
    for (fifths, pitch) in [2, 9, 4, 11, 6, 1, 8, 3, 10, 5, 0, 7]
        .into_iter()
        .enumerate()
    {
        assert_eq!(usize::from(key.fifths_above_tonic(pitch)), fifths);
        assert_eq!(usize::from(key.fifths_above_tonic(pitch + 60)), fifths);
    }
}

#[test]
fn all_signature_names_match_tonics_and_invalid_payloads_are_ignored() {
    let major = [11, 6, 1, 8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6, 1];
    let minor = [8, 3, 10, 5, 0, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];
    let minor_names = [
        "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#",
    ];
    for sharps in -7..=7 {
        let index = (sharps + 7) as usize;
        for (mode, tonics) in [(0, major), (1, minor)] {
            let key = KeySignature::from_event(&signature(0, sharps, mode)).unwrap();
            assert_eq!(key.tonic(), tonics[index]);
            if mode == 1 {
                assert_eq!(key.name(), format!("{} minor", minor_names[index]));
            }
        }
    }
    for event in [
        signature(0, -8, 0),
        signature(0, 8, 0),
        signature(0, 0, 2),
        Event {
            data: vec![0],
            ..signature(0, 0, 0)
        },
    ] {
        assert!(KeySignature::from_event(&event).is_none());
    }
}

#[test]
fn global_changes_apply_at_tick_and_format_two_keeps_tracks_independent() {
    let mut music = Music {
        format: 1,
        division: 0xe728,
        tracks: vec![
            Track {
                events: vec![
                    signature(10, -1, 1),
                    signature(20, 0, 0),
                    signature(30, 8, 0),
                ],
            },
            Track {
                events: vec![signature(20, 1, 0), signature(20, 0, 1)],
            },
        ],
    };
    let keys = KeySignatures::new(&music);
    for track in 0..2 {
        let map = keys.map(track).unwrap();
        assert!(map.at(9).is_none());
        assert_eq!(map.at(10).unwrap().name(), "D minor");
        assert_eq!(map.at(19).unwrap().name(), "D minor");
        assert_eq!(map.at(20).unwrap().name(), "A minor");
        assert_eq!(map.at(30).unwrap().name(), "A minor");
    }
    music.format = 2;
    let keys = KeySignatures::new(&music);
    assert_eq!(keys.map(0).unwrap().at(20).unwrap().name(), "C major");
    assert!(keys.map(1).unwrap().at(19).is_none());
    assert_eq!(keys.map(1).unwrap().at(20).unwrap().name(), "A minor");
    assert!(keys.map(2).is_none());
}

#[test]
fn guide_inheritance_is_per_track_and_own_key_stays_in_charge() {
    use cat_smf_editor::score::chord_guide;
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![signature(0, 0, 0)],
            },
            Track { events: vec![] },
            Track {
                events: vec![signature(3840, -1, 1)],
            },
        ],
    };
    chord_guide::add(&mut music).unwrap();
    for e in [
        signature(0, 0, 0),
        signature(1920, 1, 0),
        signature(5760, 2, 0),
    ] {
        cat_smf_editor::edit::insert(&mut music.tracks[3], e);
    }
    let keys = KeySignatures::new(&music);
    for t in [1, 2] {
        assert_eq!(keys.map(t).unwrap().at(1919).unwrap().name(), "C major");
        assert_eq!(keys.map(t).unwrap().at(1920).unwrap().name(), "G major");
    }
    assert_eq!(keys.map(1).unwrap().at(5760).unwrap().name(), "D major");
    assert_eq!(keys.map(2).unwrap().at(5760).unwrap().name(), "D minor");
    assert_eq!(keys.map(0).unwrap().at(5760).unwrap().name(), "C major");
    // Local Pattern/Scene timelines keep the prior interpretation, excluding the guide.
    assert_eq!(
        KeySignatures::local_timeline(&music)
            .map(1)
            .unwrap()
            .at(5760)
            .unwrap()
            .name(),
        "D minor"
    );
    music.tracks[2].events.clear();
    assert_eq!(
        KeySignatures::new(&music)
            .map(2)
            .unwrap()
            .at(5760)
            .unwrap()
            .name(),
        "D major"
    );
    music.tracks.pop();
    music.tracks[1].events.push(signature(0, 3, 0));
    assert_eq!(
        KeySignatures::new(&music)
            .map(2)
            .unwrap()
            .at(0)
            .unwrap()
            .name(),
        "C major"
    );
}
