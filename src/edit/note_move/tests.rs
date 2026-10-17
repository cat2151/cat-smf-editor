use super::move_note;
use crate::{
    model::{Event, Track},
    score::note_view::NoteView,
};

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

/// C-E-G arpeggio of eighth notes from 0, then another C at 960.
fn arpeggio() -> Track {
    let mut track = Track {
        events: vec![
            event(0, 0x90, &[60, 100]),
            event(240, 0x80, &[60, 0]),
            event(240, 0x90, &[64, 100]),
            event(480, 0x80, &[64, 0]),
            event(480, 0x90, &[67, 100]),
            event(720, 0x80, &[67, 0]),
            event(960, 0x90, &[60, 100]),
            event(1200, 0x80, &[60, 0]),
            eot(1200),
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

/// (start tick, key, duration) of every note, in event order.
fn notes(track: &Track) -> Vec<(u64, u8, Option<u64>)> {
    let view = NoteView::new(&track.events);
    track
        .events
        .iter()
        .zip(&view.durations)
        .filter(|(e, _)| e.status & 0xf0 == 0x90)
        .map(|(e, &d)| (e.tick, e.data[0], d))
        .collect()
}

fn deltas_match_ticks(track: &Track) {
    let mut previous = 0;
    for e in &track.events {
        assert_eq!(u64::from(e.delta), e.tick - previous);
        previous = e.tick;
    }
}

#[test]
fn moving_later_keeps_the_duration_and_every_other_event() {
    let mut track = arpeggio();
    move_note(&mut track, 2, 120).unwrap();
    assert_eq!(
        ticks(&track),
        vec![
            (0, 0x90, 60),
            (240, 0x80, 60),
            (360, 0x90, 64),
            (480, 0x90, 67),
            (600, 0x80, 64),
            (720, 0x80, 67),
            (960, 0x90, 60),
            (1200, 0x80, 60),
            (1200, 0xff, 0),
        ]
    );
    assert_eq!(
        notes(&track),
        vec![
            (0, 60, Some(240)),
            (360, 64, Some(240)),
            (480, 67, Some(240)),
            (960, 60, Some(240)),
        ]
    );
    deltas_match_ticks(&track);
}

#[test]
fn moving_earlier_keeps_the_duration_and_every_other_event() {
    let mut track = arpeggio();
    move_note(&mut track, 4, -360).unwrap();
    assert_eq!(
        ticks(&track),
        vec![
            (0, 0x90, 60),
            (120, 0x90, 67),
            (240, 0x80, 60),
            (240, 0x90, 64),
            (360, 0x80, 67),
            (480, 0x80, 64),
            (960, 0x90, 60),
            (1200, 0x80, 60),
            (1200, 0xff, 0),
        ]
    );
    assert_eq!(
        notes(&track),
        vec![
            (0, 60, Some(240)),
            (120, 67, Some(240)),
            (240, 64, Some(240)),
            (960, 60, Some(240)),
        ]
    );
    deltas_match_ticks(&track);
}

#[test]
fn moving_past_the_end_extends_only_end_of_track() {
    let mut track = arpeggio();
    move_note(&mut track, 4, 1000).unwrap();
    let last = ticks(&track);
    assert_eq!(
        last[last.len() - 3..],
        [(1480, 0x90, 67), (1720, 0x80, 67), (1720, 0xff, 0)]
    );
    assert_eq!(notes(&track)[3], (1480, 67, Some(240)));
}

#[test]
fn moving_by_zero_changes_nothing() {
    let mut track = arpeggio();
    move_note(&mut track, 2, 0).unwrap();
    assert_eq!(ticks(&track), ticks(&arpeggio()));
}

#[test]
fn crossing_a_same_key_note_is_rejected_and_leaves_the_track_unchanged() {
    // Later: the first C would start after the second C.
    let mut track = arpeggio();
    assert!(move_note(&mut track, 0, 1000).is_err());
    assert_eq!(ticks(&track), ticks(&arpeggio()));

    // Earlier: C at 600..960 would start before C at 240..480.
    let two_cs = || Track {
        events: vec![
            event(240, 0x90, &[60, 100]),
            event(480, 0x80, &[60, 0]),
            event(600, 0x90, &[60, 100]),
            event(960, 0x80, &[60, 0]),
        ],
    };
    let mut track = two_cs();
    assert!(move_note(&mut track, 2, -500).is_err());
    assert_eq!(ticks(&track), ticks(&two_cs()));
}

#[test]
fn overlapping_without_reordering_the_starts_keeps_both_lengths() {
    let mut track = arpeggio();
    move_note(&mut track, 0, 900).unwrap();
    let view = notes(&track);
    assert!(view.contains(&(900, 60, Some(240))));
    assert!(view.contains(&(960, 60, Some(240))));
}

#[test]
fn moving_before_tick_zero_is_rejected() {
    let mut track = arpeggio();
    assert!(move_note(&mut track, 0, -1).is_err());
    assert!(move_note(&mut track, 2, -241).is_err());
    assert_eq!(ticks(&track), ticks(&arpeggio()));
    // Exactly to tick 0 is allowed.
    move_note(&mut track, 2, -240).unwrap();
    assert!(notes(&track).contains(&(0, 64, Some(240))));
}

#[test]
fn only_note_ons_can_be_moved() {
    let mut track = arpeggio();
    assert!(move_note(&mut track, 1, 10).is_err());
    assert!(move_note(&mut track, 99, 10).is_err());
    assert_eq!(ticks(&track), ticks(&arpeggio()));
}
