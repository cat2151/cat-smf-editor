use super::*;
use crate::{
    model::Event,
    score::{
        key_signature::KeySignature,
        pattern::recording::{Key, Mode, Scale},
    },
};

fn signature(sharps: i8, minor: bool) -> KeySignature {
    KeySignature::from_event(&Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![sharps as u8, u8::from(minor)],
    })
    .unwrap()
}
fn c_major() -> Settings {
    Settings::new(Key::from_signature(None), 500_000)
}
fn a_minor() -> Settings {
    Settings::new(Key::from_signature(Some(signature(0, true))), 500_000)
}
fn rows(s: &Settings) -> Vec<Vec<Option<u8>>> {
    (0..4u8)
        .map(|row| {
            (0..ROW_KEYS[usize::from(row)])
                .map(|column| {
                    let p = pitches(s, Pad::Row { row, column });
                    assert!(p.len() <= 1);
                    p.first().copied()
                })
                .collect()
        })
        .collect()
}
fn some(row: &[u8]) -> Vec<Option<u8>> {
    row.iter().copied().map(Some).collect()
}
fn chord(s: &Settings, row: u8, column: u8) -> Vec<u8> {
    pitches(s, Pad::Row { row, column })
}

#[test]
fn song_key_defaults_and_bases() {
    let c = c_major();
    assert_eq!(
        c.key(),
        Key {
            tonic: 0,
            scale: Scale::MajorPentatonic
        }
    );
    assert_eq!((c.note_base, c.chord_base), (60, 48));
    let a = a_minor();
    assert_eq!(
        a.key(),
        Key {
            tonic: 9,
            scale: Scale::MinorPentatonic
        }
    );
    assert_eq!((a.note_base, a.chord_base), (69, 57));
    // E major: four sharps.
    assert_eq!(Key::from_signature(Some(signature(4, false))).tonic, 4);
}

#[test]
fn note_mode_rows_in_c_major() {
    assert_eq!(
        rows(&c_major()),
        vec![
            some(&[60, 62, 64, 67, 69, 72, 74, 76, 79, 81, 84]),
            some(&[72, 74, 76, 79, 81, 84, 86, 88, 91, 93, 96, 98]),
            some(&[84, 86, 88, 91, 93, 96, 98, 100, 103, 105, 108, 110]),
            some(&[96, 98, 100, 103, 105, 108, 110, 112, 115, 117, 120, 122, 124]),
        ]
    );
}

#[test]
fn note_mode_rows_in_a_minor_drop_notes_above_127() {
    let mut top = some(&[105, 108, 110, 112, 115, 117, 120, 122, 124, 127]);
    top.extend([None; 3]);
    assert_eq!(
        rows(&a_minor()),
        vec![
            some(&[69, 72, 74, 76, 79, 81, 84, 86, 88, 91, 93]),
            some(&[81, 84, 86, 88, 91, 93, 96, 98, 100, 103, 105, 108]),
            some(&[93, 96, 98, 100, 103, 105, 108, 110, 112, 115, 117, 120]),
            top,
        ]
    );
}

#[test]
fn root_position_modes_step_rows_by_octave() {
    let mut s = c_major();
    s.mode = Mode::Triad;
    assert_eq!(chord(&s, 0, 0), [48, 52, 55]);
    assert_eq!(chord(&s, 0, 1), [50, 53, 57]);
    assert_eq!(chord(&s, 0, 6), [59, 62, 65]);
    assert_eq!(chord(&s, 1, 0), [60, 64, 67]);
    assert_eq!(chord(&s, 2, 0), [72, 76, 79]);
    assert_eq!(chord(&s, 3, 0), [84, 88, 91]);
    s.mode = Mode::Seventh;
    assert_eq!(chord(&s, 0, 0), [48, 52, 55, 59]);
    assert_eq!(chord(&s, 0, 1), [50, 53, 57, 60]);
    assert_eq!(chord(&s, 0, 6), [59, 62, 65, 69]);
    assert_eq!(chord(&s, 1, 0), [60, 64, 67, 71]);
    assert_eq!(chord(&s, 2, 0), [72, 76, 79, 83]);
    assert_eq!(chord(&s, 3, 0), [84, 88, 91, 95]);
    // Natural minor parent scale.
    let mut a = a_minor();
    a.mode = Mode::Triad;
    a.mode = Mode::Seventh;
    assert_eq!(chord(&a, 0, 0), [57, 60, 64, 67]);
    assert_eq!(chord(&a, 0, 1), [59, 62, 65, 69]);
    assert_eq!(chord(&a, 0, 6), [55, 59, 62, 65].map(|p| p + 12));
}

#[test]
fn inversion_modes_step_rows_by_inversion() {
    let mut s = c_major();
    s.mode = Mode::TriadInversions;
    assert_eq!(chord(&s, 0, 0), [48, 52, 55]);
    assert_eq!(chord(&s, 0, 1), [50, 53, 57]);
    assert_eq!(chord(&s, 0, 6), [59, 62, 65]);
    assert_eq!(chord(&s, 1, 0), [52, 55, 60]);
    assert_eq!(chord(&s, 2, 0), [55, 60, 64]);
    assert_eq!(chord(&s, 3, 0), [60, 64, 67]);
    s.mode = Mode::SeventhInversions;
    assert_eq!(chord(&s, 0, 0), [48, 52, 55, 59]);
    assert_eq!(chord(&s, 0, 1), [50, 53, 57, 60]);
    assert_eq!(chord(&s, 0, 6), [59, 62, 65, 69]);
    assert_eq!(chord(&s, 1, 0), [52, 55, 59, 60]);
    assert_eq!(chord(&s, 2, 0), [55, 59, 60, 64]);
    assert_eq!(chord(&s, 3, 0), [59, 60, 64, 67]);
}

#[test]
fn letters_ignore_key_and_mode_and_follow_the_note_base_octave() {
    let mut s = a_minor();
    s.mode = Mode::Triad;
    let letters = |s: &Settings| {
        (0..7)
            .map(|i| pitches(s, Pad::Letter(i)))
            .collect::<Vec<_>>()
    };
    assert_eq!(letters(&s).concat(), [60, 62, 64, 65, 67, 69, 71]);
    s.note_base = 120;
    let high = letters(&s);
    assert_eq!(high[..5].concat(), [120, 122, 124, 125, 127]);
    assert!(high[5].is_empty() && high[6].is_empty());
    assert!(pitches(&s, Pad::Letter(7)).is_empty());
}

#[test]
fn notes_above_127_and_pads_outside_the_rows_are_dropped() {
    let mut s = c_major();
    s.mode = Mode::Triad;
    s.mode = Mode::Seventh;
    s.chord_base = 120;
    assert_eq!(chord(&s, 0, 0), [120, 124, 127]);
    assert!(chord(&s, 0, 11).is_empty());
    assert!(chord(&s, 3, 13).is_empty());
    assert!(chord(&s, 4, 0).is_empty());
}

#[test]
fn key_change_moves_both_bases_to_the_tonic_of_their_octave() {
    let mut s = c_major();
    s.note_base = 72;
    s.chord_base = 43;
    s.set_key(Key {
        tonic: 9,
        scale: Scale::MinorPentatonic,
    });
    assert_eq!((s.note_base, s.chord_base), (81, 45));
    s.note_base = 127;
    s.set_key(Key {
        tonic: 11,
        scale: Scale::MajorPentatonic,
    });
    assert_eq!(s.note_base, 119);
    s.set_key(Key {
        tonic: 11,
        scale: Scale::MinorPentatonic,
    });
    assert_eq!(s.note_base, 119);
}

#[test]
fn drums_play_one_lane_per_row_or_first_four_letters_and_piano_keys_their_pitch() {
    let mut s = c_major();
    s.drums = true;
    s.mode = Mode::Seventh;
    for (row, lane) in [36, 38, 42, 39].into_iter().enumerate() {
        for column in 0..ROW_KEYS[row] {
            assert_eq!(chord(&s, row as u8, column), [lane]);
        }
    }
    assert!(chord(&s, 0, 11).is_empty());
    let letters: Vec<_> = (0..7).map(|i| pitches(&s, Pad::Letter(i))).collect();
    assert_eq!(letters[..4].concat(), [36, 38, 42, 39]);
    assert!(letters[4..].iter().all(Vec::is_empty));
    assert_eq!(pitches(&s, Pad::Pitch(70)), [70]);
}
