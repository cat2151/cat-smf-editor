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
    dropouts: AtomicU64,
    dropout_us: AtomicU64,
    dropout_callback_us: AtomicU64,
    rests: AtomicU64,
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
    /// The stream stopped rendering because nothing sounds.
    pub fn rest(&self) {
        self.rests.fetch_add(1, Relaxed);
    }
    pub fn lock_miss(&self) {
        self.lock_misses.fetch_add(1, Relaxed);
    }
    fn dropout(&self, silence_us: u64, slowest_callback_us: u64) {
        self.dropouts.fetch_add(1, Relaxed);
        self.dropout_us.fetch_add(silence_us, Relaxed);
        self.dropout_callback_us
            .fetch_max(slowest_callback_us, Relaxed);
    }
}

/// Finds silence the device inserted because a callback came too late.
///
/// cpal hands each callback the free space of the device buffer, so the
/// unplayed remainder is the largest request seen minus this request. When the
/// device runs dry it plays silence without consuming submitted frames, and the
/// played frames fall behind the wall clock by the silence length.
/// The engine consumes in periods, so the lag is a sawtooth: compare the minimum
/// of consecutive windows. The short window keeps device clock drift negligible.
pub(super) struct Dropouts {
    rate: u32,
    start: Option<Instant>,
    buffer: usize,
    submitted: u64,
    callbacks: u32,
    window_lag_us: i64,
    window_callback_us: u64,
    previous: Option<(i64, u64)>,
}
impl Dropouts {
    const WINDOW: u32 = 30;
    const THRESHOLD_US: i64 = 2_000;
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            start: None,
            buffer: 0,
            submitted: 0,
            callbacks: 0,
            window_lag_us: i64::MAX,
            window_callback_us: 0,
            previous: None,
        }
    }
    /// Call at callback entry with the requested frames.
    pub fn begin(&mut self, frames: usize, now: Instant, counters: &Counters) {
        let start = *self.start.get_or_insert(now);
        if frames > self.buffer {
            // A larger request redefines the buffer size; earlier lags are not comparable.
            self.buffer = frames;
            self.previous = None;
            self.callbacks = 0;
            self.window_lag_us = i64::MAX;
            self.window_callback_us = 0;
        }
        let played = self.submitted - (self.buffer - frames) as u64;
        self.submitted += frames as u64;
        let elapsed = now.duration_since(start).as_micros() as i64;
        let lag = elapsed - (played as i128 * 1_000_000 / i128::from(self.rate)) as i64;
        self.window_lag_us = self.window_lag_us.min(lag);
        self.callbacks += 1;
        if self.callbacks < Self::WINDOW {
            return;
        }
        let window = (self.window_lag_us, self.window_callback_us);
        if let Some((lag, callback_us)) = self.previous
            && window.0 - lag >= Self::THRESHOLD_US
        {
            counters.dropout((window.0 - lag) as u64, callback_us.max(window.1));
        }
        self.previous = Some(window);
        self.callbacks = 0;
        self.window_lag_us = i64::MAX;
        self.window_callback_us = 0;
    }
    /// Call when the callback returns, with the time measured from `begin`.
    pub fn end(&mut self, since: Instant) {
        self.window_callback_us = self
            .window_callback_us
            .max(since.elapsed().as_micros() as u64);
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
        let rests = counters.rests.swap(0, Relaxed);
        if rests != 0 {
            crate::store::error_log::write(
                "audio-rest",
                &format!(
                    "pid={} stream={} count={rests}",
                    std::process::id(),
                    counters.id
                ),
            );
        }
        let dropouts = counters.dropouts.swap(0, Relaxed);
        if dropouts != 0 {
            let silence = counters.dropout_us.swap(0, Relaxed) as f64 / 1000.0;
            let callback = counters.dropout_callback_us.swap(0, Relaxed) as f64 / 1000.0;
            crate::store::error_log::write("audio-dropout", &format!(
                "pid={} stream={} count={dropouts} silence_ms={silence:.1} slowest_nearby_callback_ms={callback:.1}", std::process::id(), counters.id));
        }
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

    /// 480-frame periods in a 1056-frame buffer, as the default WASAPI device requests.
    fn run(
        dropouts: &mut Dropouts,
        counters: &Counters,
        start: Instant,
        callbacks: std::ops::Range<u64>,
        offset_us: u64,
    ) {
        for n in callbacks {
            let at = start + Duration::from_micros(n * 10_000 + offset_us);
            dropouts.begin(if n == 0 { 1056 } else { 480 }, at, counters);
        }
    }

    #[test]
    fn steady_periods_report_no_dropout_and_a_late_device_does() {
        let counters = Counters::default();
        let mut dropouts = Dropouts::new(48000);
        let start = Instant::now();
        run(&mut dropouts, &counters, start, 0..200, 0);
        assert_eq!(counters.dropouts.load(Relaxed), 0);
        // The device played 25 ms of silence and resumed with the queued frames.
        run(&mut dropouts, &counters, start, 200..400, 25_000);
        assert_eq!(counters.dropouts.load(Relaxed), 1);
        assert_eq!(counters.dropout_us.load(Relaxed), 25_000);
    }
}
