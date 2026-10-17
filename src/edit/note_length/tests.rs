use super::set_duration;
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
fn lengthening_one_chord_note_moves_only_its_note_off() {
    let mut track = chord();
    set_duration(&mut track, 1, 1440).unwrap();
    assert_eq!(
        ticks(&track),
        vec![
            (0, 0x90, 60),
            (0, 0x90, 64),
            (0, 0x90, 67),
            (960, 0x80, 60),
            (960, 0x80, 67),
            (960, 0x90, 60),
            (1440, 0x80, 64),
            (1920, 0x80, 60),
            (1920, 0xff, 0),
        ]
    );
    let deltas: Vec<u32> = track.events.iter().map(|e| e.delta).collect();
    assert_eq!(deltas, vec![0, 0, 0, 960, 0, 0, 480, 480, 0]);
    let view = crate::score::note_view::NoteView::new(&track.events);
    assert_eq!(view.durations[0], Some(960));
    assert_eq!(view.durations[1], Some(1440));
    assert_eq!(view.durations[5], Some(960));
}

#[test]
fn shortening_keeps_the_note_off_after_the_events_at_its_new_tick() {
    let mut track = chord();
    set_duration(&mut track, 2, 960).unwrap();
    assert_eq!(ticks(&track), ticks(&chord()));
    set_duration(&mut track, 0, 480).unwrap();
    assert_eq!(ticks(&track)[3], (480, 0x80, 60));
}

#[test]
fn a_note_off_past_end_of_track_moves_end_of_track_with_it() {
    let mut track = chord();
    set_duration(&mut track, 6, 1200).unwrap();
    assert_eq!(ticks(&track)[7..], [(2160, 0x80, 60), (2160, 0xff, 0)]);
}

#[test]
fn rejected_lengths_leave_the_track_unchanged() {
    for (on, duration) in [
        // Zero length.
        (1, 0),
        // Not a Note On.
        (3, 100),
    ] {
        let mut track = chord();
        assert!(set_duration(&mut track, on, duration).is_err());
        assert_eq!(ticks(&track), ticks(&chord()));
    }
}

#[test]
fn ending_after_a_later_same_key_note_ends_is_rejected() {
    // C at 240..480 and C at 600..960; ending the first C after 960 would pair it with the
    // second Note Off and change both lengths.
    let mut track = Track {
        events: vec![
            event(240, 0x90, &[60, 100]),
            event(480, 0x80, &[60, 0]),
            event(600, 0x90, &[60, 100]),
            event(960, 0x80, &[60, 0]),
        ],
    };
    let before = ticks(&track);
    assert!(set_duration(&mut track, 0, 900).is_err());
    assert_eq!(ticks(&track), before);
    // Overlapping without reordering the ends keeps both lengths.
    set_duration(&mut track, 0, 500).unwrap();
    let view = crate::score::note_view::NoteView::new(&track.events);
    assert_eq!(view.durations[0], Some(500));
    assert_eq!(view.durations[1], Some(360));
}
