//! Pitch spelling, staff steps and note values; no drawing.
use std::collections::HashMap;

/// Diatonic steps counted from C-1, so middle C is 35.
pub(super) const MIDDLE_C: i32 = 35;
/// Lowest line of the treble staff (E4).
pub(super) const TREBLE_BOTTOM: i32 = 37;
/// Lowest line of the bass staff (G2).
pub(super) const BASS_BOTTOM: i32 = 25;
/// Highest line of the treble staff (F5).
pub(super) const TREBLE_TOP: i32 = TREBLE_BOTTOM + 8;

/// Pitch class of each natural letter, C to B.
const NATURAL: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Letters in the order sharps enter a key signature: F C G D A E B.
const SHARP_ORDER: [usize; 7] = [3, 0, 4, 1, 5, 2, 6];

/// Alteration of `letter` (C = 0) under a key of `sharps` (negative for flats).
pub(super) fn key_alter(sharps: i8, letter: usize) -> i8 {
    let count = usize::from(sharps.unsigned_abs());
    if sharps > 0 && SHARP_ORDER[..count].contains(&letter) {
        1
    } else if sharps < 0 && SHARP_ORDER[7 - count..].contains(&letter) {
        -1
    } else {
        0
    }
}

/// Treble staff steps of a key signature's accidentals, in entry order.
pub(super) fn key_steps(sharps: i8) -> Vec<i32> {
    // F5 C5 G5 D5 A4 E5 B4 for sharps, B4 E5 A4 D5 G4 C5 F4 for flats.
    let (steps, count) = if sharps >= 0 {
        ([45, 42, 46, 43, 40, 44, 41], sharps as usize)
    } else {
        ([41, 44, 40, 43, 39, 42, 38], sharps.unsigned_abs() as usize)
    };
    steps[..count].to_vec()
}

/// Spelled note: staff step and alteration (-1 flat, 0 natural, 1 sharp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Spelled {
    pub step: i32,
    pub alter: i8,
}

/// In-key spelling first, then natural, then the key's accidental direction.
pub(super) fn spell(pitch: u8, sharps: i8) -> Spelled {
    let pitch = i32::from(pitch);
    let direction = if sharps >= 0 { 1 } else { -1 };
    let (letter, alter) = (0..7)
        .flat_map(|letter| (-1..=1).map(move |alter| (letter, alter)))
        .filter(|&(letter, alter)| {
            (NATURAL[letter] + i32::from(alter)).rem_euclid(12) == pitch % 12
        })
        .min_by_key(|&(letter, alter)| {
            if alter == key_alter(sharps, letter) {
                0
            } else if alter == 0 {
                1
            } else if alter == direction {
                2
            } else {
                3
            }
        })
        .expect("every pitch class has a spelling");
    Spelled {
        step: (pitch - i32::from(alter)).div_euclid(12) * 7 + letter as i32,
        alter,
    }
}

/// Accidentals hold until the barline; each measure starts from the key signature.
#[derive(Default)]
pub(super) struct Accidentals {
    measure: u128,
    alters: HashMap<i32, i8>,
}

impl Accidentals {
    /// The accidental to print before `note`, if its alteration differs from the measure so far.
    pub fn next(&mut self, measure: u128, note: Spelled, sharps: i8) -> Option<i8> {
        if measure != self.measure {
            self.measure = measure;
            self.alters.clear();
        }
        let current = self
            .alters
            .get(&note.step)
            .copied()
            .unwrap_or_else(|| key_alter(sharps, note.step.rem_euclid(7) as usize));
        self.alters.insert(note.step, note.alter);
        (current != note.alter).then_some(note.alter)
    }
}

/// Note value as a power of two of a quarter note: 2 whole, 1 half, 0 quarter, -1 eighth ... -3.
pub(super) fn value(duration: u64, ppqn: u16) -> i32 {
    let quarters = duration.max(1) as f64 / f64::from(ppqn.max(1));
    (quarters.log2().round() as i32).clamp(-3, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spells_in_key_and_out_of_key_notes() {
        assert_eq!(
            spell(60, 0),
            Spelled {
                step: MIDDLE_C,
                alter: 0
            }
        );
        assert_eq!(
            spell(61, 0),
            Spelled {
                step: MIDDLE_C,
                alter: 1
            }
        );
        assert_eq!(spell(70, -1).alter, -1); // B flat in F major
        assert_eq!(spell(66, 1).alter, 1); // F sharp in G major
        assert_eq!(spell(65, 1), Spelled { step: 38, alter: 0 }); // F natural in G major
        assert_eq!(
            spell(59, -6),
            Spelled {
                step: MIDDLE_C,
                alter: -1
            }
        ); // C flat in G flat major
        assert_eq!(spell(64, 0).step, TREBLE_BOTTOM);
        assert_eq!(spell(43, 0).step, BASS_BOTTOM);
    }

    #[test]
    fn accidentals_hold_within_a_measure() {
        let mut accidentals = Accidentals::default();
        let f_sharp = spell(66, 0);
        let f = spell(65, 0);
        assert_eq!(accidentals.next(1, f_sharp, 0), Some(1));
        assert_eq!(accidentals.next(1, f_sharp, 0), None);
        assert_eq!(accidentals.next(1, f, 0), Some(0));
        assert_eq!(accidentals.next(2, f, 0), None);
        assert_eq!(accidentals.next(3, spell(65, 1), 1), Some(0));
    }

    #[test]
    fn values_round_to_the_nearest_power_of_two() {
        assert_eq!(value(480, 480), 0);
        assert_eq!(value(400, 480), 0);
        assert_eq!(value(240, 480), -1);
        assert_eq!(value(1920, 480), 2);
        assert_eq!(value(9999, 480), 2);
        assert_eq!(value(10, 480), -3);
    }

    #[test]
    fn key_signatures_follow_the_circle_of_fifths() {
        assert_eq!(key_alter(2, 3), 1); // F in D major
        assert_eq!(key_alter(2, 0), 1); // C in D major
        assert_eq!(key_alter(2, 4), 0);
        assert_eq!(key_alter(-1, 6), -1); // B in F major
        assert_eq!(key_alter(-1, 2), 0);
        assert_eq!(key_steps(2), vec![45, 42]);
        assert_eq!(key_steps(-2), vec![41, 44]);
    }
}
