use super::events;
use crate::model::Event;
use crate::score::pattern::recording::{control, note};

fn shown(events: &[Event]) -> Vec<(u64, u8, u8, u8)> {
    events
        .iter()
        .map(|e| (e.tick, e.status, e.data[0], e.data[1]))
        .collect()
}

#[test]
fn notes_snap_and_controls_keep_their_ticks() {
    let mut played = vec![note(10, 60, 90), note(130, 64, 80)];
    control(&mut played, 50, 7);
    played.extend([note(230, 60, 0), note(250, 64, 0)]);
    assert_eq!(
        shown(&events(played, 120, None)),
        vec![
            (0, 0x90, 60, 90),
            (50, 0xb0, 1, 7),
            (120, 0x90, 64, 80),
            (240, 0x80, 60, 0),
            (240, 0x80, 64, 0),
        ]
    );
}

#[test]
fn a_note_on_the_end_of_a_repeating_take_starts_at_its_start() {
    let played = vec![note(1900, 60, 90), note(1915, 60, 0)];
    assert_eq!(
        shown(&events(played, 120, Some(1920))),
        vec![(0, 0x90, 60, 90), (120, 0x80, 60, 0)]
    );
}

#[test]
fn a_repeated_key_starts_after_the_previous_note_of_its_pitch() {
    // Both quick presses round onto the same sixteenth.
    let played = vec![
        note(0, 60, 90),
        note(20, 60, 0),
        note(40, 60, 100),
        note(50, 60, 0),
    ];
    assert_eq!(
        shown(&events(played, 120, None)),
        vec![
            (0, 0x90, 60, 90),
            (120, 0x80, 60, 0),
            (120, 0x90, 60, 100),
            (240, 0x80, 60, 0),
        ]
    );
}
