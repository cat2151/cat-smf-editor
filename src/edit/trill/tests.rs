use super::*;
use crate::model::Track;

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: data.to_vec(),
    }
}

pub(super) fn song() -> Music {
    let mut events = Vec::new();
    // Two simultaneous trills, with a sustained accompaniment on the same channel.
    for base in [60, 72] {
        for i in 0..8 {
            let pitch = base + (i % 2) as u8 * 2;
            events.push(event(100 + i * 24, 0x90, &[pitch, 90]));
            events.push(event(124 + i * 24, 0x90, &[pitch, 0]));
        }
    }
    events.push(event(100, 0x90, &[48, 80]));
    events.push(event(292, 0x80, &[48, 0]));
    events.sort_by_key(|e| e.tick);
    events.push(Event {
        tick: 400,
        delta: 0,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    });
    let mut music = Music {
        format: 1,
        division: 384,
        tracks: vec![
            Track {
                events: vec![
                    Event {
                        tick: 0,
                        delta: 0,
                        status: 0xff,
                        meta_type: 0x51,
                        data: 600_000u32.to_be_bytes()[1..].to_vec(),
                    },
                    // The first trill interval crosses a tempo change: 12 ticks at each tempo.
                    Event {
                        tick: 112,
                        delta: 0,
                        status: 0xff,
                        meta_type: 0x51,
                        data: 1_200_000u32.to_be_bytes()[1..].to_vec(),
                    },
                ],
            },
            Track { events },
        ],
    };
    for track in &mut music.tracks {
        refresh_deltas(track).unwrap();
    }
    music
}

#[test]
fn starting_pitch_can_switch_with_one_note_remaining_and_preserves_pitch_velocities() {
    let mut music = song();
    for e in &mut music.tracks[1].events {
        if note_on(e) && e.data[0] == 62 {
            e.data[1] = 77;
        }
    }
    initialize(&mut music).unwrap();
    // Selection also captures a pattern that has not yet been rebuilt.
    set_first_note(&mut music, 1, 1, true).unwrap();
    set_timing(&mut music, 1, 1, 0.05625, 10.0).unwrap();
    super::super::regenerate(&mut music).unwrap();
    for (upper, pitch, velocity) in [(true, 62, 77), (false, 60, 90), (true, 62, 77)] {
        set_first_note(&mut music, 1, 1, upper).unwrap();
        super::super::regenerate(&mut music).unwrap();
        let ons: Vec<_> = music.tracks[1]
            .events
            .iter()
            .filter(|e| note_on(e) && [60, 62].contains(&e.data[0]))
            .collect();
        assert_eq!(ons.len(), 1);
        assert_eq!(ons[0].data, [pitch, velocity]);
        assert_eq!(ons[0].tick, 100);
        assert_eq!(
            read(&music).unwrap()[0]
                .metadata
                .pattern
                .as_ref()
                .unwrap()
                .first_note,
            pitch
        );
    }
}

#[test]
fn detects_polyphonic_trills_and_captures_elapsed_seconds_once() {
    let mut music = song();
    let original = music.clone();
    initialize(&mut music).unwrap();
    let regions = read(&music).unwrap();
    assert_eq!(regions.len(), 2);
    assert_eq!((regions[0].start, regions[0].end), (100, 292));
    assert_eq!(regions[0].metadata.notes, [60, 62]);
    assert_eq!(regions[1].metadata.notes, [72, 74]);
    assert!((regions[0].metadata.interval_seconds - 0.05625).abs() < 1e-12);
    let mut stripped = music.clone();
    for track in &mut stripped.tracks {
        track.events.retain(|e| !owned(e));
        refresh_deltas(track).unwrap();
    }
    assert_eq!(format!("{stripped:?}"), format!("{original:?}"));
    music.tracks[0].events[0].data = 300_000u32.to_be_bytes()[1..].to_vec();
    initialize(&mut music).unwrap();
    assert_eq!(read(&music).unwrap(), regions);
    assert_eq!(
        music.tracks[1].events.iter().filter(|e| owned(e)).count(),
        4
    );
    let encoded = crate::smf_io::encode(&music).unwrap();
    let decoded = crate::ffi::parse(&encoded).unwrap();
    assert_eq!(read(&decoded).unwrap(), regions);
}

#[test]
fn excludes_slow_short_repeated_wide_and_unpaired_notes() {
    for case in 0..6 {
        let mut music = song();
        music.tracks[1]
            .events
            .retain(|e| e.status == 0xff || e.data[0] < 70);
        let events = &mut music.tracks[1].events;
        match case {
            0 => {
                for e in events {
                    e.tick *= 4;
                }
            }
            1 => events.retain(|e| e.status == 0xff || e.tick < 220),
            2 => {
                for e in events {
                    if e.data.first() == Some(&62) {
                        e.data[0] = 60;
                    }
                }
            }
            3 => {
                for e in events {
                    if e.data.first() == Some(&62) {
                        e.data[0] = 67;
                    }
                }
            }
            4 => events.retain(|e| !crate::score::note_view::note_off(e)),
            _ => {
                for e in events {
                    if e.status == 0x90 && e.data[1] == 0 {
                        e.tick -= 1;
                    }
                }
            }
        }
        assert!(detect(&music).unwrap().is_empty(), "case {case}");
    }
}

#[test]
fn malformed_pairs_and_speeds_are_errors_without_mutation() {
    for case in 0..4 {
        let mut music = song();
        initialize(&mut music).unwrap();
        let events = &mut music.tracks[1].events;
        let index = events
            .iter()
            .position(|e| e.data.starts_with(START))
            .unwrap();
        match case {
            0 => {
                events.remove(index);
            }
            1 => {
                let mut metadata = read(&song_with_metadata()).unwrap()[0].metadata.clone();
                metadata.interval_seconds = 0.0;
                events[index].data = START.to_vec();
                events[index]
                    .data
                    .extend(serde_json::to_vec(&metadata).unwrap());
            }
            2 => {
                let duplicate = events[index].clone();
                events.insert(index, duplicate);
            }
            _ => events.retain(|e| !e.data.starts_with(END)),
        }
        let before = format!("{music:?}");
        assert!(initialize(&mut music).is_err());
        assert_eq!(format!("{music:?}"), before);
    }
}

fn song_with_metadata() -> Music {
    let mut music = song();
    initialize(&mut music).unwrap();
    music
}

#[test]
fn no_candidates_stays_unchanged_and_unsupported_timing_is_rejected() {
    let mut music = song();
    music.tracks[1].events.clear();
    let before = format!("{music:?}");
    initialize(&mut music).unwrap();
    assert_eq!(format!("{music:?}"), before);
    for (format, division) in [(0, 384), (2, 384), (1, 0), (1, 0xe728)] {
        music.format = format;
        music.division = division;
        assert!(initialize(&mut music).is_err());
    }
}

#[test]
fn managed_notes_cannot_be_edited_directly_but_accompaniment_can() {
    let mut music = song();
    initialize(&mut music).unwrap();
    crate::edit::regenerate(&mut music).unwrap();
    let mut managed = 0;
    let mut accompaniment = 0;
    for (i, e) in music.tracks[1]
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
    {
        if e.data[0] == 48 {
            ensure_note_editable(&music, 1, i).unwrap();
            accompaniment += 1;
        } else {
            assert!(ensure_note_editable(&music, 1, i).is_err());
            managed += 1;
        }
    }
    assert!(managed > 0 && accompaniment > 0);
}
