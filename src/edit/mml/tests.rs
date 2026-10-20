use super::*;

fn music(division: u16) -> Music {
    Music {
        format: 1,
        division,
        tracks: vec![Track {
            events: vec![
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0xc5,
                    meta_type: 0,
                    data: vec![12],
                },
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    }
}

#[test]
fn generated_tracks_become_notes_at_cursor_with_scaled_ticks_and_one_channel() {
    let mut target = music(960);
    let original = target.tracks[0].events[0].clone();
    insert(&mut target, 0, 123, 5, "o5 l4 c d; o4 l2 g").unwrap();
    let notes: Vec<_> = target.tracks[0]
        .events
        .iter()
        .filter(|e| matches!(e.status & 0xf0, 0x80 | 0x90))
        .collect();
    assert_eq!(notes.len(), 6);
    assert!(notes.iter().all(|e| e.status & 15 == 5));
    let onsets: Vec<_> = notes
        .iter()
        .filter(|e| note_on(e))
        .map(|e| (e.tick, e.data[0]))
        .collect();
    assert_eq!(onsets, [(123, 60), (123, 55), (1083, 62)]);
    assert_eq!(target.tracks[0].events[0].data, original.data);
    assert_eq!(target.tracks[0].events[0].status, original.status);
    let last_note = notes.iter().map(|e| e.tick).max().unwrap();
    assert_eq!(target.tracks[0].events.last().unwrap().tick, last_note);
    let encoded = crate::smf_io::encode(&target).unwrap();
    assert_eq!(
        crate::smf_io::encode(&crate::ffi::parse(&encoded).unwrap()).unwrap(),
        encoded
    );
}

#[test]
fn chords_and_tempo_directives_insert_only_notes_and_keep_musical_time() {
    let mut fast = music(480);
    let mut slow = fast.clone();
    insert(&mut fast, 0, 0, 0, "t240 c4 d4").unwrap();
    insert(&mut slow, 0, 0, 0, "t60 c4 d4").unwrap();
    assert_eq!(
        crate::smf_io::encode(&fast).unwrap(),
        crate::smf_io::encode(&slow).unwrap()
    );
    let mut chords = music(480);
    insert(&mut chords, 0, 720, 9, "C F G").unwrap();
    assert!(
        chords.tracks[0]
            .events
            .iter()
            .filter(|e| note_on(e))
            .count()
            >= 9
    );
    assert!(chords.tracks[0]
        .events
        .iter()
        .filter(|e| note_on(e))
        .all(|e| e.tick >= 720 && e.status == 0x99));
}

#[test]
fn errors_are_atomic_and_channel_selection_is_deterministic() {
    let mut target = music(480);
    assert_eq!(channel(&target.tracks[0], None), 5);
    let before = crate::smf_io::encode(&target).unwrap();
    for (tick, text) in [(0, ""), (0, "r4"), (u64::MAX, "c4")] {
        assert!(insert(&mut target, 0, tick, 5, text).is_err());
        assert_eq!(crate::smf_io::encode(&target).unwrap(), before);
    }
    target.division = 0xe728;
    assert!(insert(&mut target, 0, 0, 5, "c4").is_err());
    assert_eq!(channel(&Track { events: vec![] }, None), 0);
}

#[test]
fn phrase_end_includes_trailing_rest_and_survives_smf_roundtrip() {
    let mut m = music(480);
    let end = insert(&mut m, 0, 120, 0, "c4 r4").unwrap();
    assert_eq!(end, 1080);
    let decoded = crate::ffi::parse(&crate::smf_io::encode(&m).unwrap()).unwrap();
    assert_eq!(decoded.tracks[0].events.last().unwrap().tick, end);
}

fn notes(music: &Music) -> Vec<(u64, u8, u64)> {
    let events = &music.tracks[0].events;
    let pairs = crate::edit::relocate::pairs(events.iter());
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .map(|(i, e)| (e.tick, e.data[0], events[pairs[i].unwrap()].tick - e.tick))
        .collect()
}

fn with_notes(list: &[(u64, u8, u64)]) -> Music {
    let mut m = music(480);
    for &(tick, key, length) in list {
        for (at, status, velocity) in [(tick, 0x90, 100), (tick + length, 0x80, 0)] {
            crate::edit::insert(
                &mut m.tracks[0],
                Event {
                    delta: 0,
                    tick: at,
                    status,
                    meta_type: 0,
                    data: vec![key, velocity],
                },
            );
        }
    }
    crate::edit::refresh_deltas(&mut m.tracks[0]).unwrap();
    m
}

#[test]
fn replace_overwrites_the_notes_under_the_phrase_and_keeps_the_rest() {
    // Measure 2 of 4/4 at 480: 1920..3840. A note from measure 1 sounds into it.
    let original = [
        (960, 50, 1440),
        (1920, 60, 480),
        (2400, 62, 480),
        (2880, 64, 480),
        (3360, 65, 480),
        (3840, 67, 480),
    ];
    let mut m = with_notes(&original);
    let end = replace(&mut m, 0, 1920, 0, "o5 l4 g a").unwrap();
    assert_eq!(end, 2880);
    assert_eq!(
        notes(&m),
        [
            (960, 50, 960),
            (1920, 67, 480),
            (2400, 69, 480),
            (2880, 64, 480),
            (3360, 65, 480),
            (3840, 67, 480),
        ]
    );
    let encoded = crate::smf_io::encode(&m).unwrap();
    assert_eq!(
        crate::smf_io::encode(&crate::ffi::parse(&encoded).unwrap()).unwrap(),
        encoded
    );
}

#[test]
fn a_rest_in_the_phrase_removes_the_notes_under_it() {
    let mut m = with_notes(&[(1920, 61, 240), (2400, 62, 480), (2880, 64, 480)]);
    let end = replace(&mut m, 0, 1920, 0, "o5 l4 c r").unwrap();
    assert_eq!(end, 2880);
    assert_eq!(notes(&m), [(1920, 60, 480), (2880, 64, 480)]);
    let mut only_rests = with_notes(&[(1920, 60, 480), (2400, 62, 480), (2880, 64, 480)]);
    assert_eq!(replace(&mut only_rests, 0, 1920, 0, "r2").unwrap(), 2880);
    assert_eq!(notes(&only_rests), [(2880, 64, 480)]);
}
