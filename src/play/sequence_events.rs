//! Event preparation runs before the audio callback. Source order is authoritative.
use super::{MidiEvent, Note, Sequence};
use cat_smf_editor::{
    model::Event,
    score::{
        audition_timing::TempoMap,
        note_view::{note_off, note_on},
    },
};
use std::collections::{HashMap, VecDeque};

pub(super) fn is_on(word: u32) -> bool {
    (word >> 20) & 15 == 9 && word & 127 != 0
}
fn key(word: u32) -> u32 {
    word & 0x000f_7f00
}

impl Sequence {
    /// Synthetic auditions have no source event stream; prepare their events once.
    pub fn new(notes: Vec<Note>, end: f64) -> Self {
        let mut events = Vec::with_capacity(notes.len() * 2);
        for note in &notes {
            events.push(MidiEvent {
                time: note.start,
                word: 0x2090_0000 | (u32::from(note.pitch) << 8) | u32::from(note.velocity),
            });
            if note.duration.is_finite() {
                events.push(MidiEvent {
                    time: note.start + note.duration,
                    word: 0x2080_0000 | (u32::from(note.pitch) << 8),
                });
            }
        }
        // Stable order preserves a zero-duration note's own on/off pair.
        events.sort_by(|a, b| a.time.total_cmp(&b.time));
        let mut result = Self {
            notes,
            end,
            events,
            origin: 0.0,
        };
        result.close_at_end();
        result
    }

    pub fn from_source(
        notes: Vec<Note>,
        end: f64,
        source: &[Event],
        selected: &[usize],
        tempo: &TempoMap,
        start: u64,
    ) -> Self {
        let mut result = Self {
            notes,
            end,
            events: Vec::new(),
            origin: 0.0,
        };
        let mut pending: HashMap<(u8, u8), VecDeque<bool>> = HashMap::new();
        result.origin = tempo.seconds_at(start);
        for (index, event) in source.iter().enumerate() {
            if !note_on(event) && !note_off(event) {
                continue;
            }
            let queue = pending
                .entry((event.status & 15, event.data[0]))
                .or_default();
            let keep = if note_on(event) {
                let keep = selected.binary_search(&index).is_ok();
                queue.push_back(keep);
                keep
            } else {
                queue.pop_front().unwrap_or(false)
            };
            if keep {
                // SMF's velocity-zero Note On means Note Off. Native CLAP notes
                // have no such alias: send an explicit Off so the plugin and our
                // held-note accounting agree, including when playback is stopped.
                let status = if note_off(event) {
                    0x80 | (event.status & 15)
                } else {
                    event.status
                };
                result.events.push(MidiEvent {
                    time: tempo.seconds_at(event.tick),
                    word: 0x2000_0000
                        | (u32::from(status) << 16)
                        | (u32::from(event.data[0]) << 8)
                        | u32::from(event.data[1]),
                });
            }
        }
        result.close_at_end();
        result
    }

    pub fn truncate(&mut self, duration: f64) {
        self.end = duration;
        self.notes.retain_mut(|note| {
            note.duration = note.duration.min((duration - note.start).max(0.0));
            note.start < duration && note.duration >= 0.0
        });
        self.close_at_end();
    }

    fn close_at_end(&mut self) {
        if !self.end.is_finite() {
            return;
        }
        let end = self.origin + self.end;
        self.events
            .retain(|event| event.time <= end && !(event.time == end && is_on(event.word)));
        let mut held: Vec<u32> = Vec::new();
        for event in &self.events {
            if is_on(event.word) {
                held.push(event.word);
            } else if let Some(index) = held.iter().position(|&word| key(word) == key(event.word)) {
                held.remove(index);
            }
        }
        self.events.extend(held.into_iter().map(|word| MidiEvent {
            time: end,
            word: 0x2080_0000 | key(word),
        }));
    }

    pub fn skip(&mut self, elapsed: f64) {
        let origin = self.origin + elapsed;
        let mut held: Vec<MidiEvent> = Vec::new();
        for event in self.events.iter().take_while(|event| event.time < origin) {
            if is_on(event.word) {
                held.push(event.clone());
            } else if let Some(index) = held.iter().position(|on| key(on.word) == key(event.word)) {
                held.remove(index);
            }
        }
        // An off at the seek point must not resurrect the note that just ended.
        for event in self
            .events
            .iter()
            .filter(|event| event.time == origin && !is_on(event.word))
        {
            if let Some(index) = held.iter().position(|on| key(on.word) == key(event.word)) {
                held.remove(index);
            }
        }
        for on in &mut held {
            on.time = origin;
        }
        held.extend(
            self.events
                .iter()
                .filter(|event| event.time >= origin)
                .cloned(),
        );
        self.events = held;
        self.notes.retain_mut(|note| {
            let end = note.start + note.duration;
            if end <= elapsed {
                return false;
            }
            if note.start < elapsed {
                note.duration = end - elapsed;
                note.start = 0.0;
            } else {
                note.start -= elapsed;
            }
            true
        });
        self.origin = origin;
        self.end = (self.end - elapsed).max(0.0);
    }
}
