//! Metronome clicks synthesized on the audio thread, which also fixes the loop origin.
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    OnceLock,
};
use std::time::{Duration, Instant};

const LENGTH: Duration = Duration::from_millis(30);
const AMPLITUDE: f32 = 0.3;
const BEATS_PER_MEASURE: u64 = 4;
/// Frames are counted only after the first click.
const IDLE: u64 = u64::MAX;

#[derive(Default)]
pub(crate) struct Click {
    /// Microseconds per quarter note, set with the request.
    tempo: AtomicU32,
    requested: AtomicBool,
    /// Frames since the first click; written by the audio thread only.
    position: AtomicU64,
    /// When the count-in ends, as seen by the audio thread.
    origin: OnceLock<Instant>,
}

impl Click {
    pub(crate) fn new() -> Self {
        Self {
            position: AtomicU64::new(IDLE),
            ..Self::default()
        }
    }

    /// The next audio block starts one measure of count-in, then keeps the beat.
    pub(crate) fn request(&self, tempo: u32) {
        if self.requested.load(Ordering::Acquire) {
            return;
        }
        self.tempo.store(tempo, Ordering::Relaxed);
        self.requested.store(true, Ordering::Release);
    }

    pub(crate) fn requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }

    /// The end of the count-in, once the audio thread has played its first click.
    pub(crate) fn origin(&self) -> Option<Instant> {
        self.origin.get().copied()
    }

    /// Adds clicks to interleaved `out`, whose first frame is rendered at `now`.
    pub(crate) fn render(&self, out: &mut [f32], channels: usize, rate: u32, now: Instant) {
        let tempo = self.tempo.load(Ordering::Relaxed);
        let mut position = self.position.load(Ordering::Relaxed);
        if position == IDLE {
            if !self.requested.load(Ordering::Acquire) || tempo == 0 {
                return;
            }
            // Output latency is not compensated.
            let measure = Duration::from_micros(u64::from(tempo) * BEATS_PER_MEASURE);
            if self.origin.set(now + measure).is_err() {
                return;
            }
            position = 0;
        }
        let beat = f64::from(tempo) * f64::from(rate) / 1e6;
        let length = (LENGTH.as_secs_f64() * f64::from(rate)) as u64;
        let channels = channels.max(1);
        for (n, frame) in out.chunks_mut(channels).enumerate() {
            let p = position + n as u64;
            let mut k = (p as f64 / beat).floor() as u64;
            if ((k as f64) * beat).round() as u64 > p {
                k -= 1;
            } else if (((k + 1) as f64) * beat).round() as u64 <= p {
                k += 1;
            }
            let offset = p - ((k as f64) * beat).round() as u64;
            if offset >= length {
                continue;
            }
            let pitch = if k.is_multiple_of(BEATS_PER_MEASURE) {
                1760.0
            } else {
                880.0
            };
            let t = (offset + 1) as f32 / rate as f32;
            let envelope = 1.0 - offset as f32 / length as f32;
            let sample = AMPLITUDE * envelope * (std::f32::consts::TAU * pitch * t).sin();
            for s in frame {
                *s += sample;
            }
        }
        self.position
            .store(position + (out.len() / channels) as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "audition_click_tests.rs"]
mod tests;
