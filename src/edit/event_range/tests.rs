use super::*;
fn event(tick: u64, status: u8, value: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: if matches!(status & 0xf0, 0xc0 | 0xd0) {
            vec![value]
        } else {
            vec![value, 100]
        },
    }
}
#[test]
fn notes_delete_paired_off_without_moving_survivors() {
    let mut t = Track {
        events: vec![
            event(0, 0x90, 60),
            event(10, 0x90, 62),
            event(20, 0x80, 60),
            event(30, 0x80, 62),
        ],
    };
    delete(&mut t, &[0], true).unwrap();
    assert_eq!(
        t.events.iter().map(|e| e.tick).collect::<Vec<_>>(),
        [10, 30, 30]
    );
    assert_eq!(t.events[0].data[0], 62);
    assert_eq!(t.events[1].delta, 20);
}
#[test]
fn raw_deletes_only_selected_events() {
    let mut t = Track {
        events: vec![event(0, 0x90, 60), event(20, 0x80, 60)],
    };
    delete(&mut t, &[0], false).unwrap();
    assert_eq!(t.events[0].status, 0x80);
}
fn key(tick: u64, minor: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![0xff, minor],
    }
}

#[test]
fn initialization_inherits_only_whole_events_common_to_every_other_track() {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xc0, 9), event(0, 0x90, 60), event(480, 0x80, 60)],
            },
            Track {
                events: vec![
                    key(10, 1),
                    event(10, 0xc0, 1),
                    event(10, 0x90, 60),
                    event(20, 0xb0, 2),
                ],
            },
            Track {
                events: vec![key(0, 1), event(0, 0xc0, 4), event(0, 0x90, 60)],
            },
        ],
    };
    let before: Vec<_> = m.tracks.iter().skip(1).map(|t| t.events.clone()).collect();
    initialize(&mut m, 0).unwrap();
    assert_eq!(m.tracks[0].events.len(), 2);
    let e = &m.tracks[0].events[0];
    assert_eq!(
        (e.status, e.meta_type, e.data.as_slice()),
        (0xff, 0x59, &[0xff, 1][..])
    );
    assert!(m.tracks[0]
        .events
        .iter()
        .all(|e| e.tick == 0 && e.delta == 0));
    for (track, original) in m.tracks.iter().skip(1).zip(before) {
        assert_eq!(track.events.len(), original.len());
        for (e, old) in track.events.iter().zip(original) {
            assert_eq!(
                (e.tick, e.status, e.meta_type, &e.data),
                (old.tick, old.status, old.meta_type, &old.data)
            );
        }
    }
    let decoded = crate::ffi::parse(&crate::smf_io::encode(&m).unwrap()).unwrap();
    assert_eq!(decoded.tracks[0].events[0].data, [0xff, 1]);
}

#[test]
fn byte_difference_or_missing_event_prevents_inheritance() {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: vec![key(0, 1), event(0, 0xc0, 4)],
            },
            Track {
                events: vec![key(0, 0), event(0, 0xc1, 4)],
            },
        ],
    };
    initialize(&mut m, 0).unwrap();
    assert_eq!(m.tracks[0].events.len(), 1);
    assert!(super::super::end_of_track(&m.tracks[0].events[0]));
    m.tracks[2].events = vec![key(0, 1)];
    m.tracks.push(Track { events: vec![] });
    initialize(&mut m, 0).unwrap();
    assert_eq!(
        m.tracks[0].events.len(),
        1,
        "empty source track shares no initial events"
    );
}

#[test]
fn initialization_retains_common_duplicate_count_and_resets_with_no_other_track() {
    let cc = event(0, 0xb0, 7);
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track { events: vec![] },
            Track {
                events: vec![cc.clone(), cc.clone()],
            },
            Track { events: vec![cc] },
        ],
    };
    initialize(&mut m, 0).unwrap();
    assert_eq!(m.tracks[0].events.len(), 2);
    m.tracks.truncate(1);
    initialize(&mut m, 0).unwrap();
    assert_eq!(m.tracks[0].events.len(), 1);
}
