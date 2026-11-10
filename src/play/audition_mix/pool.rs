//! Helper threads that render a block's parts alongside the thread that asks for the block.
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// One part's work for the current block; called once per part index.
type Job<'a> = &'a (dyn Fn(usize) + Sync);

/// `next` packs the generation (high 32 bits), the part count (16 bits) and the next
/// unclaimed part (low 16 bits), so a claim can never mix two blocks.
const PART_BITS: u32 = 16;
const PART_MASK: u64 = (1 << PART_BITS) - 1;
const MAX_PARTS: usize = PART_MASK as usize;

struct Shared {
    next: AtomicU64,
    /// Points at the publisher's `Job`, valid while any part of its generation is unfinished.
    job: AtomicPtr<()>,
    done: AtomicUsize,
    stop: AtomicBool,
    /// While set, the spinning helper parks like the others until the next `run`.
    rest: AtomicBool,
    /// Whether the spinning helper is parked for a rest; read by tests only.
    #[cfg(test)]
    parked: AtomicBool,
}
impl Shared {
    fn claim(&self) -> Option<usize> {
        let mut packed = self.next.load(Ordering::Acquire);
        loop {
            let index = (packed & PART_MASK) as usize;
            let count = (packed >> PART_BITS & PART_MASK) as usize;
            if index >= count {
                return None;
            }
            match self.next.compare_exchange_weak(
                packed,
                packed + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(index),
                Err(now) => packed = now,
            }
        }
    }

    fn work(&self) {
        while let Some(index) = self.claim() {
            // SAFETY: the claim succeeded in the current generation, so its publisher is
            // still waiting for this part and the `Job` it points at is alive.
            let job = unsafe { *(self.job.load(Ordering::Acquire) as *const Job) };
            job(index);
            self.done.fetch_add(1, Ordering::Release);
        }
    }
}

pub(super) struct Pool {
    shared: Arc<Shared>,
    helpers: Vec<JoinHandle<()>>,
    generation: u64,
}
impl Pool {
    pub fn new(helpers: usize) -> Self {
        let shared = Arc::new(Shared {
            next: AtomicU64::new(0),
            job: AtomicPtr::new(std::ptr::null_mut()),
            done: AtomicUsize::new(0),
            stop: AtomicBool::new(false),
            rest: AtomicBool::new(false),
            #[cfg(test)]
            parked: AtomicBool::new(false),
        });
        // Between blocks the first helper waits by spinning instead of parking. With every
        // participant idle for most of each period the plugins ran about 2.5 times slower,
        // likely because the clock all cores share dropped; one busy core prevents it.
        // A resting stream renders nothing, so that helper parks too.
        let helpers = (0..helpers)
            .filter_map(|index| {
                let shared = shared.clone();
                std::thread::Builder::new()
                    .name("audio-part".into())
                    .spawn(move || {
                        while !shared.stop.load(Ordering::Acquire) {
                            shared.work();
                            if index == 0 && !shared.rest.load(Ordering::Acquire) {
                                std::hint::spin_loop();
                            } else {
                                #[cfg(test)]
                                if index == 0 {
                                    shared.parked.store(true, Ordering::Release);
                                }
                                std::thread::park();
                                #[cfg(test)]
                                if index == 0 {
                                    shared.parked.store(false, Ordering::Release);
                                }
                            }
                        }
                    })
                    .ok()
            })
            .collect();
        Self {
            shared,
            helpers,
            generation: 0,
        }
    }

    /// Helper threads plus the calling thread, which always takes part.
    #[cfg(test)]
    pub fn participants(&self) -> usize {
        self.helpers.len() + 1
    }

    /// Lets the spinning helper park until the next `run`.
    pub fn rest(&self) {
        self.shared.rest.store(true, Ordering::Release);
    }

    /// Whether the spinning helper is parked for a rest.
    #[cfg(test)]
    pub fn parked(&self) -> bool {
        self.shared.parked.load(Ordering::Acquire)
    }

    /// Runs `job` once for every index in `0..count` and returns after all have finished.
    pub fn run(&mut self, count: usize, job: Job) {
        if self.shared.rest.swap(false, Ordering::AcqRel)
            && let Some(helper) = self.helpers.first()
        {
            helper.thread().unpark();
        }
        if count <= 1 || count > MAX_PARTS || self.helpers.is_empty() {
            (0..count).for_each(job);
            return;
        }
        let shared = &*self.shared;
        self.generation = self.generation.wrapping_add(1) & u64::from(u32::MAX);
        shared.done.store(0, Ordering::Relaxed);
        shared
            .job
            .store(&job as *const Job as *mut (), Ordering::Relaxed);
        shared.next.store(
            self.generation << 32 | (count as u64) << PART_BITS,
            Ordering::Release,
        );
        for helper in self.helpers.iter().take(count - 1).skip(1) {
            helper.thread().unpark();
        }
        shared.work();
        while shared.done.load(Ordering::Acquire) < count {
            std::hint::spin_loop();
        }
    }

    /// Lets the helpers and the calling thread preempt ordinary work, as an audio device needs.
    pub fn boost(&self) {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            boost(None);
            for helper in &self.helpers {
                boost(Some(helper.as_raw_handle()));
            }
        }
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        for helper in &self.helpers {
            helper.thread().unpark();
        }
        for helper in self.helpers.drain(..) {
            let _ = helper.join();
        }
    }
}

/// `None` raises the calling thread.
#[cfg(windows)]
fn boost(thread: Option<std::os::windows::io::RawHandle>) {
    use std::ffi::c_void;
    const THREAD_PRIORITY_TIME_CRITICAL: i32 = 15;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
    }
    // SAFETY: the handle is the calling thread's pseudo handle or a live helper's handle.
    unsafe {
        SetThreadPriority(
            thread.unwrap_or_else(|| GetCurrentThread()),
            THREAD_PRIORITY_TIME_CRITICAL,
        );
    }
}
