//! Callback counters are sampled on the UI thread; no file I/O in the callback.
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Counters {
    id: u64,
    blocks: AtomicU64,
    generation: AtomicU64,
    peak: AtomicU64,
    nonfinite: AtomicU64,
    failures: AtomicU64,
    lock_misses: AtomicU64,
    track_peaks: Vec<AtomicU64>,
}
impl Counters {
    pub fn new(tracks: usize) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, Relaxed),
            track_peaks: (0..tracks).map(|_| AtomicU64::new(0)).collect(),
            ..Default::default()
        }
    }
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn track(&self, index: usize, samples: &[f32]) {
        if samples.iter().any(|s| !s.is_finite()) {
            self.failure(8);
        }
        if let Some(peak) = self.track_peaks.get(index) {
            let value = samples
                .iter()
                .filter(|s| s.is_finite())
                .fold(0.0_f32, |p, s| p.max(s.abs()));
            peak.fetch_max(u64::from(value.to_bits()), Relaxed);
        }
    }
    pub fn block(&self, generation: u64, samples: &[f32]) {
        self.blocks.fetch_add(1, Relaxed);
        self.generation.store(generation, Relaxed);
        let mut peak = 0.0_f32;
        let mut nonfinite = 0;
        for &sample in samples {
            if sample.is_finite() {
                peak = peak.max(sample.abs());
            } else {
                nonfinite += 1;
            }
        }
        self.peak.fetch_max(u64::from(peak.to_bits()), Relaxed);
        self.nonfinite.fetch_add(nonfinite, Relaxed);
    }
    pub fn failure(&self, stage: u64) {
        self.failures.fetch_or(stage, Relaxed);
    }
    pub fn lock_miss(&self) {
        self.lock_misses.fetch_add(1, Relaxed);
    }
}

pub(super) struct Monitor {
    last: Instant,
    generation: u64,
}
impl Default for Monitor {
    fn default() -> Self {
        Self {
            last: Instant::now(),
            generation: u64::MAX,
        }
    }
}
impl Monitor {
    pub fn poll(&mut self, counters: &Counters, expected: u64, force: bool) {
        if !force && self.last.elapsed() < Duration::from_secs(1) {
            return;
        }
        let interval = self.last.elapsed().as_millis();
        self.last = Instant::now();
        let blocks = counters.blocks.swap(0, Relaxed);
        let seen = counters.generation.load(Relaxed);
        let peak = f32::from_bits(counters.peak.swap(0, Relaxed) as u32);
        let nonfinite = counters.nonfinite.swap(0, Relaxed);
        let failures = counters.failures.swap(0, Relaxed);
        let misses = counters.lock_misses.swap(0, Relaxed);
        let tracks: Vec<_> = counters
            .track_peaks
            .iter()
            .map(|peak| f32::from_bits(peak.swap(0, Relaxed) as u32))
            .collect();
        if force
            || expected != self.generation
            || peak > 0.0001
            || nonfinite != 0
            || failures != 0
            || misses != 0
            || blocks == 0
        {
            crate::store::error_log::write("audio-diagnostic", &format!(
                "pid={} stream={} interval_ms={interval} expected_generation={expected} callback_generation={seen} blocks={blocks} output_peak={peak:.6} track_peaks={tracks:?} nonfinite={nonfinite} failure_bits={failures} render_lock_misses={misses}", std::process::id(), counters.id));
        }
        self.generation = expected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures_peak_nonfinite_generation_and_failure_stages() {
        let counters = Counters::default();
        counters.block(4, &[0.1, -2.0, f32::NAN]);
        counters.block(5, &[0.5]);
        counters.failure(1);
        counters.failure(4);
        assert_eq!(counters.blocks.load(Relaxed), 2);
        assert_eq!(counters.generation.load(Relaxed), 5);
        assert_eq!(f32::from_bits(counters.peak.load(Relaxed) as u32), 2.0);
        assert_eq!(counters.nonfinite.load(Relaxed), 1);
        assert_eq!(counters.failures.load(Relaxed), 5);
    }
}
