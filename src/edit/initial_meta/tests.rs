use super::*;
use crate::model::Track;

fn meta(tick: u64, meta_type: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type,
        data: data.to_vec(),
    }
}

fn end(tick: u64) -> Event {
    meta(tick, 0x2f, &[])
}

/// Format 1: a conductor with a tempo and a track with one note, extra events added per case.
fn song(conductor: Vec<Event>, second: Vec<Event>) -> Music {
    let mut tracks = vec![
        Track {
            events: [
                vec![meta(0, 0x51, &[7, 0xa1, 0x20])],
                conductor,
                vec![end(960)],
            ]
            .concat(),
        },
        Track {
            events: [
                second,
                vec![
                    Event {
                        tick: 0,
                        delta: 0,
                        status: 0x90,
                        meta_type: 0,
                        data: vec![60, 80],
                    },
                    Event {
                        tick: 480,
                        delta: 0,
                        status: 0x80,
                        meta_type: 0,
                        data: vec![60, 0],
                    },
                    end(960),
                ],
            ]
            .concat(),
        },
    ];
    for track in &mut tracks {
        track.events.sort_by_key(|e| e.tick);
        refresh_deltas(track).unwrap();
    }
    Music {
        format: 1,
        division: 480,
        tracks,
    }
}

/// (track, tick, meta type, data) of every Time Signature and Key Signature.
fn signatures(music: &Music) -> Vec<(usize, u64, u8, Vec<u8>)> {
    music
        .tracks
        .iter()
        .enumerate()
        .flat_map(|(t, track)| track.events.iter().map(move |e| (t, e)))
        .filter(|(_, e)| e.status == 0xff && matches!(e.meta_type, 0x58 | 0x59))
        .map(|(t, e)| (t, e.tick, e.meta_type, e.data.clone()))
        .collect()
}

#[test]
fn a_song_without_either_gets_both_once_on_the_conductor_at_tick_0() {
    let mut music = song(vec![], vec![]);
    let second = format!("{:?}", music.tracks[1]);
    initialize(&mut music).unwrap();
    let want = vec![
        (0, 0, 0x58, FOUR_FOUR.to_vec()),
        (0, 0, 0x59, C_MAJOR.to_vec()),
    ];
    assert_eq!(signatures(&music), want);
    let conductor = &music.tracks[0].events;
    assert_eq!(
        conductor.last().map(|e| (e.tick, e.meta_type)),
        Some((960, 0x2f))
    );
    assert_eq!(
        conductor.iter().map(|e| e.delta).collect::<Vec<_>>(),
        [0, 0, 0, 960]
    );
    assert_eq!(format!("{:?}", music.tracks[1]), second);
    let once = format!("{music:?}");
    initialize(&mut music).unwrap();
    assert_eq!(format!("{music:?}"), once);
}

#[test]
fn only_the_missing_one_is_added_wherever_the_other_is_at_tick_0() {
    let three_four = meta(0, 0x58, &[3, 2, 24, 8]);
    let d_major = meta(0, 0x59, &[2, 0]);
    let cases = [
        (vec![three_four.clone()], vec![], 0x59),
        (vec![d_major.clone()], vec![], 0x58),
        (vec![], vec![three_four], 0x59),
        (vec![], vec![d_major], 0x58),
    ];
    for (conductor, second, added) in cases {
        let mut music = song(conductor, second);
        let before = signatures(&music);
        initialize(&mut music).unwrap();
        let after = signatures(&music);
        assert_eq!(after.len(), before.len() + 1, "{after:?}");
        let new: Vec<_> = after.iter().filter(|s| !before.contains(s)).collect();
        assert_eq!(new.len(), 1, "{after:?}");
        assert_eq!((new[0].0, new[0].1, new[0].2), (0, 0, added));
    }
}

#[test]
fn a_later_or_unreadable_signature_does_not_count_as_one_at_tick_0() {
    let cases = [
        vec![meta(480, 0x58, &[3, 2, 24, 8]), meta(480, 0x59, &[2, 0])],
        vec![meta(0, 0x58, &[0, 2, 24, 8]), meta(0, 0x59, &[9, 0])],
    ];
    for extra in cases {
        let mut music = song(extra.clone(), vec![]);
        initialize(&mut music).unwrap();
        let added: Vec<_> = signatures(&music)
            .into_iter()
            .filter(|s| {
                !extra
                    .iter()
                    .any(|e| (e.tick, e.meta_type, &e.data) == (s.1, s.2, &s.3))
            })
            .collect();
        assert_eq!(
            added,
            [
                (0, 0, 0x58, FOUR_FOUR.to_vec()),
                (0, 0, 0x59, C_MAJOR.to_vec())
            ]
        );
    }
}

#[test]
fn formats_other_than_1_are_refused_without_changes() {
    for format in [0, 2] {
        let mut music = song(vec![], vec![]);
        music.format = format;
        let before = format!("{music:?}");
        assert!(initialize(&mut music).is_err());
        assert_eq!(format!("{music:?}"), before);
    }
}
