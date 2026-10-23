//! Audio-frame scheduling, independent of UI repaint timing.
#[derive(Clone, Debug)]
pub struct Note {
    pub start: f64,
    pub duration: f64,
    pub pitch: u8,
    pub velocity: u8,
}
#[derive(Clone)]
pub struct Sequence {
    pub notes: Vec<Note>,
    pub end: f64,
    /// Absolute song times; notes above are a view for singing and transport only.
    pub events: Vec<MidiEvent>,
    pub origin: f64,
}

#[derive(Clone, Debug)]
pub struct MidiEvent {
    pub time: f64,
    pub word: u32,
}

#[path = "sequence_events.rs"]
mod events;

#[derive(Clone, Default)]
pub struct Cursor {
    pub frame: u64,
    pub next: usize,
    pub finished: bool,
}
impl Cursor {
    #[cfg(test)]
    pub fn block(
        &mut self,
        sequence: &Sequence,
        rate: u32,
        max: usize,
        velocities: &mut [u8; 128],
        durations: &mut [u64; 128],
    ) -> (usize, i32) {
        self.block_aligned(sequence, rate, max, velocities, durations, 0.0)
    }
    pub fn block_aligned(
        &mut self,
        sequence: &Sequence,
        rate: u32,
        max: usize,
        velocities: &mut [u8; 128],
        durations: &mut [u64; 128],
        origin: f64,
    ) -> (usize, i32) {
        let rounded = |s: f64| (s * f64::from(rate)).round().max(0.0) as u64;
        let frames = |s: f64| rounded(origin + s).saturating_sub(rounded(origin));
        velocities.fill(0);
        durations.fill(0);
        let mut changed = 0;
        while let Some(note) = sequence.notes.get(self.next) {
            if frames(note.start) > self.frame {
                break;
            }
            let pitch = usize::from(note.pitch);
            if pitch < 128 {
                velocities[pitch] = velocities[pitch].max(note.velocity);
                let duration = if note.duration.is_infinite() {
                    u64::MAX
                } else {
                    frames(note.start + note.duration).saturating_sub(frames(note.start))
                };
                durations[pitch] = durations[pitch].max(duration);
                changed = 2;
            }
            self.next += 1;
        }
        let end = frames(sequence.end);
        if self.frame >= end && !self.finished {
            self.finished = true;
            velocities.fill(0);
            changed = 3;
        }
        let next = sequence
            .notes
            .get(self.next)
            .map_or(end, |n| frames(n.start))
            .min(end);
        let count = if next > self.frame {
            max.min((next - self.frame).min(usize::MAX as u64) as usize)
        } else {
            max
        };
        self.frame = self.frame.saturating_add(count as u64);
        (count, changed)
    }
}
