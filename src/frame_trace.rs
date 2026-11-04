//! Opt-in frame measurements: `CAT_SMF_FRAME_TRACE=<file>` writes one JSON line per
//! `App::update`, which `scripts/frame_report.py` summarizes. Spans are UI-thread only.
use eframe::egui;
use serde::Serialize;
use std::{
    cell::RefCell,
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
};

const FLUSH_EVERY: Duration = Duration::from_secs(1);

thread_local! {
    static TRACE: RefCell<Option<Trace<BufWriter<File>>>> = const { RefCell::new(None) };
}

struct Trace<W> {
    out: W,
    start: Instant,
    flushed: Instant,
    previous_end: Option<Instant>,
    frame: Frame,
}

/// Times are microseconds. `gap_us` is the time since the previous update ended: the
/// root window's paint and swap, then any wait for a repaint. `previous_cpu_us` is
/// eframe's time for the previous frame: update and root paint, without the root swap.
/// A `discarded` pass is a layout retry: egui runs another update before painting root.
#[derive(Default, Serialize)]
struct Frame {
    t_us: u64,
    gap_us: u64,
    update_us: u64,
    previous_cpu_us: u64,
    thread_cpu_us: u64,
    discarded: bool,
    tick: Option<u64>,
    spans: Vec<(&'static str, u64)>,
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

impl<W: Write> Trace<W> {
    fn new(out: W, now: Instant) -> Self {
        Self {
            out,
            start: now,
            flushed: now,
            previous_end: None,
            frame: Frame::default(),
        }
    }

    fn begin(&mut self, now: Instant, previous_cpu: Option<f32>, thread_cpu: u64) {
        self.frame = Frame {
            t_us: micros(now - self.start),
            gap_us: self.previous_end.map_or(0, |end| micros(now - end)),
            previous_cpu_us: previous_cpu.map_or(0, |s| micros(Duration::from_secs_f32(s))),
            thread_cpu_us: thread_cpu,
            ..Frame::default()
        };
    }

    fn end(&mut self, now: Instant, discarded: bool, tick: Option<u64>) -> std::io::Result<()> {
        self.frame.update_us = micros(now - self.start) - self.frame.t_us;
        self.frame.discarded = discarded;
        self.frame.tick = tick;
        self.previous_end = Some(now);
        serde_json::to_writer(&mut self.out, &self.frame)?;
        self.out.write_all(b"\n")?;
        if now - self.flushed >= FLUSH_EVERY {
            self.flushed = now;
            self.out.flush()?;
        }
        Ok(())
    }
}

pub(crate) fn init() {
    let Some(path) = std::env::var_os("CAT_SMF_FRAME_TRACE") else {
        return;
    };
    match File::create(&path) {
        Ok(file) => TRACE.set(Some(Trace::new(BufWriter::new(file), Instant::now()))),
        Err(error) => crate::store::error_log::write("frame-trace", &error.to_string()),
    }
}

pub(crate) fn begin(frame: &eframe::Frame) {
    TRACE.with_borrow_mut(|trace| {
        if let Some(trace) = trace {
            trace.begin(Instant::now(), frame.info().cpu_usage, thread_cpu_us());
        }
    });
}

/// `tick` is the playing position this frame shows; it runs only while tracing.
pub(crate) fn end(ctx: &egui::Context, tick: impl FnOnce() -> Option<u64>) {
    TRACE.with_borrow_mut(|slot| {
        if let Some(trace) = slot
            && let Err(error) = trace.end(Instant::now(), ctx.will_discard(), tick())
        {
            crate::store::error_log::write("frame-trace", &error.to_string());
            *slot = None;
        }
    });
}

/// Times its scope into the current frame. `a/b` names a part of `a`.
#[must_use]
pub(crate) fn span(name: &'static str) -> Span {
    let start = TRACE.with_borrow(Option::is_some).then(Instant::now);
    Span { name, start }
}

pub(crate) struct Span {
    name: &'static str,
    start: Option<Instant>,
}

impl Drop for Span {
    fn drop(&mut self) {
        let Some(start) = self.start else { return };
        let took = micros(start.elapsed());
        TRACE.with_borrow_mut(|trace| {
            if let Some(trace) = trace {
                trace.frame.spans.push((self.name, took));
            }
        });
    }
}

/// CPU the UI thread has used, in the OS's coarse ticks. A frame that takes long
/// with little CPU was waiting rather than computing.
#[cfg(windows)]
fn thread_cpu_us() -> u64 {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadTimes(
            thread: *mut c_void,
            creation: *mut u64,
            exit: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> i32;
    }
    let [mut creation, mut exit, mut kernel, mut user] = [0; 4];
    // SAFETY: each pointer is a live 8-byte FILETIME; the pseudo handle needs no closing.
    let ok = unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    };
    if ok == 0 {
        0
    } else {
        (kernel + user) / 10
    }
}

#[cfg(not(windows))]
fn thread_cpu_us() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_one_json_line_with_its_times_and_spans() {
        let start = Instant::now();
        let mut trace = Trace::new(Vec::new(), start);
        trace.begin(start + Duration::from_millis(2), Some(0.003), 40);
        trace.frame.spans.push(("song", 500));
        trace
            .end(start + Duration::from_millis(5), false, Some(960))
            .unwrap();
        trace.begin(start + Duration::from_millis(9), None, 41);
        trace
            .end(start + Duration::from_millis(10), true, None)
            .unwrap();

        let lines = String::from_utf8(trace.out).unwrap();
        let lines: Vec<serde_json::Value> = lines
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines[0]["t_us"], 2000);
        assert_eq!(lines[0]["gap_us"], 0);
        assert_eq!(lines[0]["update_us"], 3000);
        assert_eq!(lines[0]["thread_cpu_us"], 40);
        assert_eq!(lines[0]["discarded"], false);
        assert_eq!(lines[0]["tick"], 960);
        assert_eq!(lines[0]["spans"], serde_json::json!([["song", 500]]));
        assert_eq!(lines[1]["gap_us"], 4000);
        assert_eq!(lines[1]["discarded"], true);
        assert_eq!(lines[1]["tick"], serde_json::Value::Null);
        assert_eq!(lines[1]["spans"], serde_json::json!([]));
    }

    #[cfg(windows)]
    #[test]
    fn busy_work_shows_in_the_thread_cpu_time() {
        let before = thread_cpu_us();
        let started = Instant::now();
        while thread_cpu_us() == before {
            assert!(started.elapsed() < Duration::from_secs(5));
        }
    }
}
