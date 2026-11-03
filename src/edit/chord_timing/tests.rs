use super::{apply, notes, onsets, Change};
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

/// C-E at 0 lasting 240, then G at 480 lasting 240.
fn track() -> Track {
    let mut t = Track {
        events: vec![
            event(0, 0x90, &[60, 100]),
            event(0, 0x90, &[64, 100]),
            event(240, 0x80, &[60, 0]),
            event(240, 0x80, &[64, 0]),
            event(480, 0x90, &[67, 100]),
            event(720, 0x80, &[67, 0]),
        ],
    };
    crate::edit::refresh_deltas(&mut t).unwrap();
    t
}

/// `(start, end, pitch)` of each note.
fn spans(t: &Track) -> Vec<(u64, u64, u8)> {
    let offs = crate::edit::relocate::pairs(t.events.iter());
    let mut out: Vec<_> = t
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| Some((e.tick, t.events[offs[i]?].tick, e.data[0])))
        .collect();
    out.sort();
    out
}

#[test]
fn chords_are_the_notes_sharing_an_onset() {
    let t = track();
    assert_eq!(onsets(&t), vec![0, 480]);
    assert_eq!(notes(&t, 0), vec![0, 1]);
    assert_eq!(notes(&t, 480), vec![4]);
    assert!(notes(&t, 240).is_empty());
}

#[test]
fn moving_keeps_every_length_and_leaves_other_chords() {
    let mut t = track();
    assert_eq!(apply(&mut t, 0, Change::Move(120)), Ok(120));
    assert_eq!(
        spans(&t),
        vec![(120, 360, 60), (120, 360, 64), (480, 720, 67)]
    );
    assert_eq!(apply(&mut t, 120, Change::Move(-120)), Ok(0));
    assert_eq!(spans(&t), spans(&track()));
}

#[test]
fn length_moves_only_the_note_off() {
    let mut t = track();
    assert_eq!(apply(&mut t, 480, Change::Length(-120)), Ok(480));
    assert_eq!(apply(&mut t, 0, Change::Length(120)), Ok(0));
    assert_eq!(spans(&t), vec![(0, 360, 60), (0, 360, 64), (480, 600, 67)]);
}

#[test]
fn quantize_snaps_the_chord_and_reports_its_new_onset() {
    let mut t = track();
    apply(&mut t, 480, Change::Move(-70)).unwrap();
    assert_eq!(apply(&mut t, 410, Change::Quantize(120)), Ok(360));
    assert_eq!(spans(&t)[2], (360, 600, 67));
}

#[test]
fn impossible_changes_leave_the_track_unchanged() {
    let mut t = track();
    assert!(apply(&mut t, 0, Change::Move(-1)).is_err());
    assert!(apply(&mut t, 480, Change::Length(-240)).is_err());
    assert!(apply(&mut t, 100, Change::Move(120)).is_err());
    assert_eq!(spans(&t), spans(&track()));
}
