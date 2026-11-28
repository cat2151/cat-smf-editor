//! [`Job::run`] with a wall-clock stamp per block, an optional wait before the first block
//! and optional pacing at the device's speed, to tell plugin load time from render speed.
use super::*;
use std::time::{Duration, Instant};

/// All Sound Off to every instrument, as a stream does before parking its plugins.
pub(crate) fn silence(instruments: &Instruments) {
    let mut out = [0.0; 2048];
    for native in &instruments.natives {
        assert!(native.silence(&mut out));
    }
}

/// Interleaved stereo and, per 1024-frame block, the time since `since` it was rendered.
pub(crate) fn run_timed(
    mut job: Job,
    since: Instant,
    wait: Duration,
    paced: bool,
) -> (Vec<f32>, Vec<Duration>) {
    std::thread::sleep(wait);
    let failed = AtomicBool::new(false);
    let mut mixer = Mixer::new(job.rate, CHANNELS, job.renders.len(), job.master);
    let mut out = Vec::with_capacity(job.frames as usize * CHANNELS);
    let mut stamps = Vec::new();
    let started = Instant::now();
    while mixer.elapsed < job.frames {
        if paced {
            let due = Duration::from_secs_f64(mixer.elapsed as f64 / f64::from(job.rate));
            if let Some(ahead) = due.checked_sub(started.elapsed()) {
                std::thread::sleep(ahead);
            }
        }
        let max = (job.frames - mixer.elapsed).min(1024) as usize;
        out.extend_from_slice(mixer.block(&mut job.renders, &job.command, max, &failed));
        stamps.push(since.elapsed());
        assert!(!failed.load(Ordering::Relaxed), "plugin failed");
    }
    (out, stamps)
}
