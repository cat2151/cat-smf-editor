use super::{note, notes, sixteenth, snap};
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

#[test]
fn notes_snap_to_the_nearest_sixteenth_and_last_at_least_one() {
    assert_eq!(sixteenth(480), Ok(120));
    assert!(sixteenth(2).is_err());
    assert_eq!(snap(59, 120), 0);
    assert_eq!(snap(60, 120), 120);
    assert_eq!(note(130, 350, 120), (120, 360));
    assert_eq!(note(118, 140, 120), (120, 240));
}

#[test]
fn only_selected_notes_move() {
    let mut t = Track {
        events: vec![
            event(10, 0x90, &[60, 100]),
            event(130, 0x90, &[64, 100]),
            event(230, 0x80, &[60, 0]),
            event(250, 0x80, &[64, 0]),
            event(250, 0xff, &[]),
        ],
    };
    t.events[4].meta_type = 0x2f;
    // A Note Off and other events in the selection are not notes of their own.
    notes(&mut t, &[0, 2, 4], 120).unwrap();
    let got: Vec<_> = t
        .events
        .iter()
        .map(|e| (e.tick, e.data.first().copied()))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, Some(60)),
            (130, Some(64)),
            (240, Some(60)),
            (250, Some(64)),
            (250, None),
        ]
    );
}
