//! Whether a stream may skip its plugins: nothing is left to play and its output has
//! stayed silent long enough for every release and effect tail to have ended.
use super::{Command, Lane};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// The audio thread's latest rest decision, read by the UI without a lock: the command
/// generation the stream rests on, plus one, or 0 while it renders.
#[derive(Default)]
pub(crate) struct Resting(AtomicU64);
impl Resting {
    pub fn store(&self, generation: u64, resting: bool) {
        let value = if resting {
            generation.wrapping_add(1)
        } else {
            0
        };
        self.0.store(value, Ordering::Relaxed);
    }

    /// False for a command the audio thread has not rendered yet: it wakes the stream.
    pub fn on(&self, generation: u64) -> bool {
        let value = self.0.load(Ordering::Relaxed);
        value != 0 && value == generation.wrapping_add(1)
    }
}

/// -90 dBFS: output below it counts as silence.
pub(super) const THRESHOLD: f32 = 3.16e-5;

pub(super) struct Rest {
    /// Silent frames since the last input or audible output.
    quiet: u64,
    /// Silent frames after which the stream rests.
    pub limit: u64,
    asleep: bool,
}
impl Rest {
    /// Rests after one second of silence.
    pub fn new(rate: u32) -> Self {
        Self {
            quiet: 0,
            limit: u64::from(rate),
            asleep: false,
        }
    }

    pub fn resting(&self, waiting: bool) -> bool {
        !waiting && self.quiet >= self.limit
    }

    /// Marks a skipped block; true for the first of a rest.
    pub fn skip(&mut self) -> bool {
        !std::mem::replace(&mut self.asleep, true)
    }

    /// Counts a rendered block's final output.
    pub fn heard(&mut self, waiting: bool, out: &[f32], frames: usize) {
        self.asleep = false;
        if waiting || out.iter().any(|s| s.abs() >= THRESHOLD) {
            self.quiet = 0;
        } else {
            self.quiet = self.quiet.saturating_add(frames as u64);
        }
    }
}

/// Whether any part still has something to play: an unfinished song part, a held pitch,
/// queued keyboard words or a CC1 change, a held key, a repeating phrase or a metronome.
pub(super) fn waiting(command: &Command, lanes: &[Lane], now: Instant) -> bool {
    command.parts.iter().zip(lanes).any(|(part, lane)| {
        ((part.timeline.is_some() || part.sequence.is_some()) && !lane.state.cursor.finished)
            || part.velocities.iter().any(|&v| v != 0)
            || part
                .live
                .as_ref()
                .is_some_and(|keys| keys.lock().unwrap().waiting(now))
            || lane.held.any()
            || part.phrase.as_ref().is_some_and(|p| p.playing())
            || part.click.as_ref().is_some_and(|c| c.requested())
    })
}

/// The pitches a part's live words left sounding.
#[derive(Clone, Copy)]
pub(super) struct Held([bool; 128]);
impl Default for Held {
    fn default() -> Self {
        Self([false; 128])
    }
}
impl Held {
    /// Follows one MIDI 1.0 channel voice word; Note On of velocity 0 releases.
    pub fn note(&mut self, word: u32) {
        let pitch = (word >> 8 & 0x7f) as usize;
        match word >> 20 & 0xf {
            0x9 => self.0[pitch] = word & 0x7f != 0,
            0x8 => self.0[pitch] = false,
            _ => {}
        }
    }

    pub fn any(&self) -> bool {
        self.0.contains(&true)
    }
}
