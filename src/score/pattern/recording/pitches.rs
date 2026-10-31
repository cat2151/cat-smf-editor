//! Which notes a recording pad sounds under the session settings.
use super::settings::Settings;
use crate::score::drums;

/// Keys in the `z`, `a`, `q` and `1` rows of a Japanese keyboard.
pub const ROW_KEYS: [u8; 4] = [11, 12, 12, 13];

const WHITE: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pad {
    /// `row` 0 is the `z` row, 3 the `1` row.
    Row { row: u8, column: u8 },
    /// 0..7 = C..B.
    Letter(u8),
    /// One pitch, from a clicked piano key.
    Pitch(u8),
}

/// Notes from low to high; notes outside 0..=127 are dropped.
pub fn pitches(s: &Settings, pad: Pad) -> Vec<u8> {
    let notes = match pad {
        Pad::Pitch(pitch) => vec![i32::from(pitch)],
        // Each row, or the first four letters, plays one drum lane.
        Pad::Letter(index) if s.drums => drums::NOTES
            .get(usize::from(index))
            .map_or(vec![], |&p| vec![i32::from(p)]),
        Pad::Letter(index) if usize::from(index) < WHITE.len() => {
            vec![i32::from(s.note_base / 12 * 12 + WHITE[usize::from(index)])]
        }
        Pad::Row { row, column } if ROW_KEYS.get(usize::from(row)).is_some_and(|&n| column < n) => {
            if s.drums {
                return vec![drums::NOTES[usize::from(row)]];
            }
            let (row, column) = (i32::from(row), i32::from(column));
            match s.mode.chord_notes() {
                None => vec![degree(
                    s.key().scale.steps(),
                    s.key().tonic,
                    s.note_base,
                    column + 5 * row,
                )],
                Some(tones) => chord(s, tones as i32, row, column),
            }
        }
        _ => vec![],
    };
    notes
        .into_iter()
        .filter_map(|p| u8::try_from(p).ok().filter(|&p| p < 128))
        .collect()
}

fn chord(s: &Settings, tones: i32, row: i32, column: i32) -> Vec<i32> {
    let scale = s.key().scale.diatonic();
    let mut notes = (0..tones)
        .map(|k| degree(scale, s.key().tonic, s.chord_base, column + 2 * k))
        .collect::<Vec<_>>();
    if !s.mode.inversions() {
        notes.iter_mut().for_each(|p| *p += 12 * row);
    } else if row >= tones {
        // A triad has no third inversion, so its top row repeats root position an octave up.
        notes.iter_mut().for_each(|p| *p += 12);
    } else {
        notes[..row as usize].iter_mut().for_each(|p| *p += 12);
    }
    notes.sort_unstable();
    notes
}

/// The `offset`-th scale note counted upward from `base`.
fn degree(steps: &[u8], tonic: u8, base: u8, offset: i32) -> i32 {
    let above = (i32::from(base) - i32::from(tonic)).rem_euclid(12);
    let start = steps.iter().rposition(|&s| i32::from(s) <= above).unwrap();
    let root = i32::from(base) - above;
    let n = start as i32 + offset;
    let len = steps.len() as i32;
    root + 12 * n.div_euclid(len) + i32::from(steps[n.rem_euclid(len) as usize])
}

#[cfg(test)]
mod tests;
