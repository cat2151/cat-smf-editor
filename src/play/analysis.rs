//! The output as the spectrum window hears it. Every output stream taps its mix
//! into a queue of its own; one worker sums the queues and keeps the latest frames.
use cat_smf_editor::{lissajous, spectrum};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const CAPACITY: usize = 16_384;
/// A paused or stopped stream stops delivering; after this it no longer holds back the sum.
const IDLE: Duration = Duration::from_millis(100);

/// Bounded single-producer/single-consumer ring; the audio thread only touches atomics.
struct Queue {
    slots: Box<[AtomicU64]>,
    read: AtomicUsize,
    write: AtomicUsize,
    rate: u32,
}

impl Queue {
    fn new(rate: u32) -> Self {
        Self {
            slots: (0..CAPACITY).map(|_| AtomicU64::new(0)).collect(),
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
            rate,
        }
    }

    fn push(&self, [left, right]: [f32; 2]) {
        let write = self.write.load(Ordering::Relaxed);
        if write.wrapping_sub(self.read.load(Ordering::Acquire)) >= CAPACITY {
            return;
        }
        self.slots[write % CAPACITY].store(
            u64::from(left.to_bits()) | (u64::from(right.to_bits()) << 32),
            Ordering::Relaxed,
        );
        self.write.store(write.wrapping_add(1), Ordering::Release);
    }

    fn pop(&self) -> Option<[f32; 2]> {
        let read = self.read.load(Ordering::Relaxed);
        if read == self.write.load(Ordering::Acquire) {
            return None;
        }
        let bits = self.slots[read % CAPACITY].load(Ordering::Relaxed);
        self.read.store(read.wrapping_add(1), Ordering::Release);
        Some([
            f32::from_bits(bits as u32),
            f32::from_bits((bits >> 32) as u32),
        ])
    }
}

struct Registry {
    /// Streams tap only while a worker reads.
    enabled: AtomicBool,
    queues: Mutex<Vec<Weak<Queue>>>,
}

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| Registry {
        enabled: AtomicBool::new(false),
        queues: Mutex::new(Vec::new()),
    })
}

/// The audio callback's end of one queue; dropping the stream drops it.
pub struct Tap(Arc<Queue>);

impl Tap {
    pub fn register(rate: u32) -> Self {
        let queue = Arc::new(Queue::new(rate));
        registry()
            .queues
            .lock()
            .unwrap()
            .push(Arc::downgrade(&queue));
        Self(queue)
    }

    /// Mono output is heard as both channels; channels past the second are not.
    pub fn write(&self, interleaved: &[f32], channels: usize) {
        if channels == 0 || !registry().enabled.load(Ordering::Relaxed) {
            return;
        }
        for frame in interleaved.chunks_exact(channels) {
            self.0.push([frame[0], *frame.get(1).unwrap_or(&frame[0])]);
        }
    }
}

struct Source {
    queue: Arc<Queue>,
    staged: VecDeque<[f32; 2]>,
    last: Instant,
}

impl Source {
    fn drain(&mut self, now: Instant) {
        let before = self.staged.len();
        while let Some(stereo) = self.queue.pop() {
            if self.staged.len() == CAPACITY {
                self.staged.pop_front();
            }
            self.staged.push_back(stereo);
        }
        if self.staged.len() != before {
            self.last = now;
        }
    }
}

/// Sums the frames every live source has delivered; idle sources are dropped from the sum.
fn mix(sources: &mut [Source], now: Instant) -> Vec<[f32; 2]> {
    for source in sources.iter_mut() {
        if now - source.last > IDLE {
            source.staged.clear();
        }
    }
    let live = || sources.iter().filter(|s| now - s.last <= IDLE);
    let Some(frames) = live().map(|s| s.staged.len()).min() else {
        return Vec::new();
    };
    let mut mixed = vec![[0.0; 2]; frames];
    for source in sources.iter_mut().filter(|s| now - s.last <= IDLE) {
        for (sum, [left, right]) in mixed.iter_mut().zip(source.staged.drain(..frames)) {
            sum[0] += left;
            sum[1] += right;
        }
    }
    mixed
}

#[derive(Default)]
struct Shared {
    stop: AtomicBool,
    spectrum: Mutex<Option<Arc<spectrum::Display>>>,
    lissajous: Mutex<Option<Arc<lissajous::Display>>>,
}

/// Reads the taps while the window is open; dropping it stops the taps.
pub struct Worker {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start() -> Self {
        registry().enabled.store(true, Ordering::Relaxed);
        let shared = Arc::new(Shared::default());
        let worker_shared = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("output analysis".into())
            .spawn(move || run(&worker_shared))
            .expect("cannot start the output analysis thread");
        Self {
            shared,
            thread: Some(thread),
        }
    }

    pub fn spectrum(&self) -> Option<Arc<spectrum::Display>> {
        self.shared.spectrum.try_lock().ok()?.clone()
    }

    pub fn lissajous(&self) -> Option<Arc<lissajous::Display>> {
        self.shared.lissajous.try_lock().ok()?.clone()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        registry().enabled.store(false, Ordering::Relaxed);
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(shared: &Shared) {
    let mut sources: Vec<Source> = Vec::new();
    let mut analyzers: Option<(u32, spectrum::Analyzer, lissajous::Analyzer)> = None;
    let mut last_publish = Instant::now();
    while !shared.stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        // Only this worker still holds the queue of a dropped stream.
        sources.retain(|s| Arc::strong_count(&s.queue) > 1);
        {
            let mut queues = registry().queues.lock().unwrap();
            queues.retain(|q| q.strong_count() > 0);
            for queue in queues.iter().filter_map(Weak::upgrade) {
                if !sources.iter().any(|s| Arc::ptr_eq(&s.queue, &queue)) {
                    sources.push(Source {
                        queue,
                        staged: VecDeque::new(),
                        last: now,
                    });
                }
            }
        }
        for source in &mut sources {
            source.drain(now);
        }
        let rate = sources
            .iter()
            .filter(|s| now - s.last <= IDLE)
            .map(|s| s.queue.rate)
            .next();
        let frames = mix(&mut sources, now);
        if let Some(rate) = rate
            && analyzers.as_ref().is_none_or(|(r, _, _)| *r != rate)
        {
            analyzers = Some((
                rate,
                spectrum::Analyzer::new(rate),
                lissajous::Analyzer::new(rate),
            ));
        }
        if let Some((_, spectrum, lissajous)) = &mut analyzers {
            for stereo in frames {
                spectrum.push(stereo);
                lissajous.push(stereo);
            }
            if last_publish.elapsed() >= Duration::from_millis(50) {
                if let Some(frame) = spectrum.display() {
                    *shared.spectrum.lock().unwrap() = Some(Arc::new(frame));
                }
                if let Some(frame) = lissajous.display() {
                    *shared.lissajous.lock().unwrap() = Some(Arc::new(frame));
                }
                last_publish = Instant::now();
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests;
