//! Opt-in startup measurements. The audio thread only publishes atomic timestamps.
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};

static TRACE: OnceLock<Trace> = OnceLock::new();

struct Trace {
    start: Instant,
    main_start_unix_us: u64,
    file: PathBuf,
    events: Mutex<Vec<Event>>,
    callback: AtomicU64,
    audible: AtomicU64,
    published: AtomicU64,
    restored: AtomicU64,
    quit: bool,
}

#[derive(Clone, Serialize)]
struct Event {
    name: &'static str,
    start_us: u64,
    duration_us: u64,
}

pub fn init() {
    let start = Instant::now();
    if let Some(file) = std::env::var_os("CAT_SMF_STARTUP_TRACE") {
        let _ = TRACE.set(Trace {
            start,
            main_start_unix_us: micros(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap(),
            ),
            file: file.into(),
            events: Mutex::new(Vec::new()),
            callback: AtomicU64::new(0),
            audible: AtomicU64::new(0),
            published: AtomicU64::new(0),
            restored: AtomicU64::new(0),
            quit: std::env::var_os("CAT_SMF_STARTUP_TRACE_QUIT").is_some(),
        });
    }
}

fn micros(duration: std::time::Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

pub struct Span {
    trace: Option<&'static Trace>,
    name: &'static str,
    start: Option<Instant>,
}

pub fn span(name: &'static str) -> Span {
    let trace = TRACE
        .get()
        .filter(|t| t.published.load(Ordering::Relaxed) < 3);
    Span {
        trace,
        name,
        start: trace.map(|_| Instant::now()),
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let (Some(trace), Some(start)) = (self.trace, self.start) {
            trace.events.lock().unwrap().push(Event {
                name: self.name,
                start_us: micros(start.duration_since(trace.start)),
                duration_us: micros(start.elapsed()),
            });
        }
    }
}

/// Called only for blocks with a sequence command, after mixing into the output.
pub fn audio_pending() -> bool {
    TRACE
        .get()
        .is_some_and(|t| t.audible.load(Ordering::Relaxed) == 0)
}

pub fn audio_block(samples: &[f32]) {
    let Some(trace) = TRACE.get() else { return };
    if trace.audible.load(Ordering::Relaxed) != 0 {
        return;
    }
    let now = micros(trace.start.elapsed()).max(1);
    let _ = trace
        .callback
        .compare_exchange(0, now, Ordering::Relaxed, Ordering::Relaxed);
    if samples.iter().any(|s| s.is_finite() && s.abs() > 1e-7) {
        let _ = trace
            .audible
            .compare_exchange(0, now, Ordering::Relaxed, Ordering::Relaxed);
    }
}

/// File I/O stays on the UI thread, after the callback has begun playback.
pub fn publish() {
    let Some(trace) = TRACE.get() else { return };
    let callback = trace.callback.load(Ordering::Relaxed);
    let audible = trace.audible.load(Ordering::Relaxed);
    let restored = trace.restored.load(Ordering::Relaxed);
    let version = if restored != 0 && audible != 0 {
        3
    } else if audible != 0 {
        2
    } else if callback != 0 {
        1
    } else {
        return;
    };
    if trace.published.load(Ordering::Relaxed) == version {
        return;
    }
    let events = trace.events.lock().unwrap().clone();
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "main_start_unix_us": trace.main_start_unix_us,
            "first_sequence_callback_us": callback,
            "first_nonzero_output_us": audible,
            "deferred_restore_completed_us": restored,
            "events": events,
        }))?;
        std::fs::write(&trace.file, bytes)?;
        Ok(())
    })();
    if result.is_ok() {
        trace.published.store(version, Ordering::Relaxed);
    }
}

pub fn restored() {
    if let Some(trace) = TRACE.get() {
        trace
            .restored
            .store(micros(trace.start.elapsed()).max(1), Ordering::Relaxed);
    }
}

pub fn quit_requested() -> bool {
    TRACE
        .get()
        .is_some_and(|t| t.quit && t.published.load(Ordering::Relaxed) == 3)
}

#[cfg(test)]
mod native_audio;
#[cfg(test)]
mod native_costs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "opens actual installed default instrument; no audio device or notes"]
    fn native_startup_host_and_chain_costs() {
        let _library =
            crate::native_library::load_at(&crate::storage_paths::cache_directory().unwrap())
                .unwrap();
        unsafe extern "C" fn wake(_: *mut std::ffi::c_void) {}
        for _ in 0..3 {
            let start = Instant::now();
            let host = unsafe { crate::host::Host::new(wake, std::ptr::null_mut()) }.unwrap();
            println!("empty host: {:?}", start.elapsed());
            drop(host);
            let start = Instant::now();
            let chain = crate::chain::Chain::open(48000, &Default::default()).unwrap();
            println!("default chain: {:?}", start.elapsed());
            drop(chain);
        }
    }
}
