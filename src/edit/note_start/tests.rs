use super::move_start;
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

/// C-E-G chord at 0 held to 960, then another C at 960.
fn chord() -> Track {
    let mut track = Track {
        events: vec![
            event(0, 0x90, &[60, 100]),
            event(0, 0x90, &[64, 100]),
            event(0, 0x90, &[67, 100]),
            event(960, 0x80, &[60, 0]),
            event(960, 0x80, &[64, 0]),
            event(960, 0x80, &[67, 0]),
            event(960, 0x90, &[60, 100]),
            event(1920, 0x80, &[60, 0]),
            eot(1920),
        ],
    };
    crate::edit::refresh_deltas(&mut track).unwrap();
    track
}

fn ticks(track: &Track) -> Vec<(u64, u8, u8)> {
    track
        .events
        .iter()
        .map(|e| (e.tick, e.status, e.data.first().copied().unwrap_or(0)))
        .collect()
}

#[test]
fn moving_one_chord_note_keeps_every_other_event_and_its_note_off() {
    let mut track = chord();
    move_start(&mut track, 1, 240).unwrap();
    assert_eq!(
        ticks(&track),
        vec![
            (0, 0x90, 60),
            (0, 0x90, 67),
            (240, 0x90, 64),
            (960, 0x80, 60),
            (960, 0x80, 64),
            (960, 0x80, 67),
            (960, 0x90, 60),
            (1920, 0x80, 60),
            (1920, 0xff, 0),
        ]
    );
    let deltas: Vec<u32> = track.events.iter().map(|e| e.delta).collect();
    assert_eq!(deltas, vec![0, 0, 240, 720, 0, 0, 0, 960, 0]);
    let view = crate::score::note_view::NoteView::new(&track.events);
    assert_eq!(view.durations[0], Some(960));
    assert_eq!(view.durations[1], Some(960));
    assert_eq!(view.durations[2], Some(720));
    assert_eq!(view.durations[6], Some(960));
}

#[test]
fn moving_to_an_occupied_tick_goes_after_the_events_there() {
    let mut track = chord();
    move_start(&mut track, 2, 0).unwrap();
    assert_eq!(
        ticks(&track)[..3],
        [(0, 0x90, 60), (0, 0x90, 64), (0, 0x90, 67)]
    );
}

#[test]
fn rejected_moves_leave_the_track_unchanged() {
    for (on, tick) in [
        // At or after its own Note Off.
        (1, 960),
        // Not a Note On.
        (3, 100),
    ] {
        let mut track = chord();
        assert!(move_start(&mut track, on, tick).is_err());
        assert_eq!(ticks(&track), ticks(&chord()));
    }
}

#[test]
fn crossing_a_same_key_note_is_rejected() {
    // C at 240..480 and C at 600..960; starting the second C before the first would pair it
    // with the first Note Off and change both lengths.
    let mut track = Track {
        events: vec![
            event(240, 0x90, &[60, 100]),
            event(480, 0x80, &[60, 0]),
            event(600, 0x90, &[60, 100]),
            event(960, 0x80, &[60, 0]),
        ],
    };
    let before = ticks(&track);
    assert!(move_start(&mut track, 2, 100).is_err());
    assert_eq!(ticks(&track), before);
    // Overlapping without reordering the starts keeps both lengths.
    move_start(&mut track, 2, 300).unwrap();
    let view = crate::score::note_view::NoteView::new(&track.events);
    assert_eq!(view.durations[0], Some(240));
    assert_eq!(view.durations[1], Some(660));
}
