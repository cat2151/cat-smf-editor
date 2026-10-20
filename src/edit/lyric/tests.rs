use super::{set, text_at};
use crate::model::{Event, Track};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

fn lyric(tick: u64, text: &str) -> Event {
    event(tick, 0xff, 0x05, text.as_bytes())
}

fn track(events: Vec<Event>) -> Track {
    let mut track = Track { events };
    crate::edit::refresh_deltas(&mut track).unwrap();
    track
}

fn summary(track: &Track) -> Vec<(u64, u32, u8, Vec<u8>)> {
    track
        .events
        .iter()
        .map(|e| (e.tick, e.delta, e.status, e.data.clone()))
        .collect()
}

#[test]
fn a_lyric_goes_just_before_its_note_and_replaces_the_ticks_lyrics() {
    let mut t = track(vec![
        event(0, 0x90, 0, &[60, 100]),
        event(480, 0x80, 0, &[60, 0]),
        lyric(480, "ど"),
        event(480, 0xb0, 0, &[7, 100]),
        event(480, 0x90, 0, &[64, 100]),
        lyric(480, "x"),
        event(960, 0x80, 0, &[64, 0]),
        event(960, 0xff, 0x2f, &[]),
    ]);
    set(&mut t, 0, "ら").unwrap();
    assert_eq!(text_at(&t.events, 0).as_deref(), Some("ら"));
    assert_eq!(
        summary(&t)[..2],
        [
            (0, 0, 0xff, "ら".as_bytes().to_vec()),
            (0, 0, 0x90, vec![60, 100]),
        ]
    );

    set(&mut t, 5, "ん").unwrap();
    assert_eq!(
        summary(&t),
        vec![
            (0, 0, 0xff, "ら".as_bytes().to_vec()),
            (0, 0, 0x90, vec![60, 100]),
            (480, 480, 0x80, vec![60, 0]),
            (480, 0, 0xb0, vec![7, 100]),
            (480, 0, 0xff, "ん".as_bytes().to_vec()),
            (480, 0, 0x90, vec![64, 100]),
            (960, 480, 0x80, vec![64, 0]),
            (960, 0, 0xff, vec![]),
        ]
    );

    set(&mut t, 5, "").unwrap();
    assert_eq!(text_at(&t.events, 480), None);
    assert_eq!(t.events.len(), 7);
    assert_eq!(t.events[4].status, 0x90);
}

#[test]
fn only_a_note_on_takes_a_lyric_and_empty_lyrics_are_skipped_when_read() {
    let mut t = track(vec![
        lyric(0, ""),
        lyric(0, "ど"),
        event(0, 0x80, 0, &[60, 0]),
    ]);
    assert_eq!(text_at(&t.events, 0).as_deref(), Some("ど"));
    assert_eq!(set(&mut t, 2, "ら"), Err("Not a Note On".into()));
    assert_eq!(set(&mut t, 9, "ら"), Err("Not a Note On".into()));
}
