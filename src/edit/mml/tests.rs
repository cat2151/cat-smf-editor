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
