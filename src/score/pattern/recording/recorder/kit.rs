//! Kit mapping stays runtime-only; saved notes always use actual MIDI pitches.
use super::*;

impl Recorder {
    pub fn set_kit_rows(&mut self, rows: Option<[Option<u8>; 48]>) {
        self.kit_rows = rows;
    }

    pub fn set_kit_chromatic(&mut self, mapping: Option<(u8, [bool; 128])>) {
        self.kit_chromatic = mapping;
    }

    pub fn pitches(&self, pad: Pad) -> Vec<u8> {
        if self.settings.mode == super::super::Mode::Chromatic
            && let Some((base, available)) = &self.kit_chromatic
        {
            let pitch = match pad {
                Pad::Pitch(p) => return (p < 128).then_some(p).into_iter().collect(),
                Pad::Row { row, column }
                    if super::super::ROW_KEYS
                        .get(usize::from(row))
                        .is_some_and(|&n| column < n) =>
                {
                    u16::from(*base)
                        + super::super::ROW_KEYS[..usize::from(row)]
                            .iter()
                            .map(|&n| u16::from(n))
                            .sum::<u16>()
                        + u16::from(column)
                }
                _ => return vec![],
            };
            return (pitch < 128 && available[usize::from(pitch)])
                .then_some(pitch as u8)
                .into_iter()
                .collect();
        }
        let Some(rows) = self.kit_rows else {
            return pitches(&self.settings, pad);
        };
        let notes = match pad {
            Pad::Pitch(p) => vec![p],
            Pad::Letter(i) => rows
                .get(usize::from(i))
                .copied()
                .flatten()
                .into_iter()
                .collect(),
            Pad::Row { row, column } => {
                if super::super::ROW_KEYS
                    .get(usize::from(row))
                    .is_some_and(|&n| column < n)
                {
                    rows[super::super::ROW_KEYS[..usize::from(row)]
                        .iter()
                        .map(|&n| usize::from(n))
                        .sum::<usize>()
                        + usize::from(column)]
                    .into_iter()
                    .collect()
                } else {
                    vec![]
                }
            }
        };
        notes.into_iter().filter(|&p| p < 128).collect()
    }

    /// End an earlier kit hit before a new strike. Earlier keys retain their held state,
    /// but relinquish ownership, so their later releases cannot stop the new hit.
    pub fn release_replaced(&mut self, pad: Pad, at: Duration) -> Result<Vec<u8>, String> {
        if self.kit_rows.is_none() || self.held(pad) {
            return Ok(vec![]);
        }
        let mut stopped = vec![];
        for pitch in self.pitches(pad) {
            if self.sounding(pitch) {
                for (_, notes) in &mut self.held {
                    notes.retain(|&p| p != pitch);
                }
                self.sounding[usize::from(pitch)] = 0;
                self.record(pitch, 0, at)?;
                stopped.push(pitch);
            }
        }
        Ok(stopped)
    }
}
