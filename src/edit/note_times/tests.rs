use super::retime;
use crate::model::{Event, Track};

fn event(tick: u64, status: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: data.to_vec(),
    }
}

fn eot(tick: u64) -> Event {
    Event {
        delta: 0,
        tick,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    }
}

fn track(events: Vec<Event>) -> Track {
    let mut t = Track { events };
    crate::edit::refresh_deltas(&mut t).unwrap();
    t
}

fn ticks(track: &Track) -> Vec<(u64, u8, u8)> {
    track
        .events
        .iter()
        .map(|e| (e.tick, e.status, e.data.first().copied().unwrap_or(0)))
        .collect()
}

#[test]
fn a_chord_moves_together_and_other_events_keep_their_ticks_and_order() {
    let mut t = track(vec![
        event(10, 0x90, &[60, 100]),
        event(10, 0x90, &[64, 100]),
        event(100, 0xb0, &[1, 64]),
        event(470, 0x80, &[60, 0]),
        event(470, 0x80, &[64, 0]),
        eot(470),
    ]);
    retime(&mut t, &[(0, 0, 480), (1, 0, 480)]).unwrap();
    assert_eq!(
        ticks(&t),
        vec![
            (0, 0x90, 60),
            (0, 0x90, 64),
            (100, 0xb0, 1),
            (480, 0x80, 60),
            (480, 0x80, 64),
            (480, 0xff, 0),
        ]
    );
    assert_eq!(t.events[2].delta, 100);
}

#[test]
fn a_note_ending_where_the_next_of_its_pitch_starts_ends_first() {
    let mut t = track(vec![
        event(0, 0x90, &[60, 100]),
        event(230, 0x80, &[60, 0]),
        event(240, 0x90, &[60, 90]),
        event(480, 0x80, &[60, 0]),
        eot(480),
    ]);
    retime(&mut t, &[(0, 0, 240)]).unwrap();
    assert_eq!(
        ticks(&t),
        vec![
            (0, 0x90, 60),
            (240, 0x80, 60),
            (240, 0x90, 60),
            (480, 0x80, 60),
            (480, 0xff, 0),
        ]
    );
}

#[test]
fn crossing_a_note_of_the_same_pitch_leaves_the_track_unchanged() {
    let mut t = track(vec![
        event(0, 0x90, &[60, 100]),
        event(100, 0x80, &[60, 0]),
        event(200, 0x90, &[60, 100]),
        event(300, 0x80, &[60, 0]),
        eot(300),
    ]);
    let before = t.clone();
    assert!(retime(&mut t, &[(0, 210, 260)]).is_err());
    assert_eq!(ticks(&t), ticks(&before));
    assert!(retime(&mut t, &[(0, 50, 50)]).is_err());
    assert!(retime(&mut t, &[(1, 0, 10)]).is_err());
    assert_eq!(ticks(&t), ticks(&before));
}

#[test]
fn a_note_moved_past_the_end_moves_end_of_track() {
    let mut t = track(vec![
        event(0, 0x90, &[60, 100]),
        event(100, 0x80, &[60, 0]),
        eot(100),
    ]);
    retime(&mut t, &[(0, 0, 960)]).unwrap();
    assert_eq!(ticks(&t).last(), Some(&(960, 0xff, 0)));
}
