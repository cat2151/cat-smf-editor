//! Every frame the output streams deliver, kept whole for tests that judge a playback.
use super::{registry, Queue};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};

/// Enables the taps while alive; streams opened later are recorded after the earlier ones.
pub(crate) struct Recorder {
    queues: Vec<Arc<Queue>>,
    /// Stereo frames in delivery order.
    pub frames: Vec<[f32; 2]>,
    /// Where each stream's frames begin in `frames`.
    pub starts: Vec<usize>,
    pub rate: u32,
}

impl Recorder {
    pub fn start() -> Self {
        registry().enabled.store(true, Ordering::Relaxed);
        Self {
            queues: Vec::new(),
            frames: Vec::new(),
            starts: Vec::new(),
            rate: 0,
        }
    }

    /// Takes what every stream has delivered since the last call.
    pub fn drain(&mut self) {
        let known = self.queues.len();
        for queue in registry()
            .queues
            .lock()
            .unwrap()
            .iter()
            .filter_map(Weak::upgrade)
        {
            if !self.queues.iter().any(|q| Arc::ptr_eq(q, &queue)) {
                self.rate = queue.rate;
                self.queues.push(queue);
            }
        }
        for (index, queue) in self.queues.iter().enumerate() {
            if index >= known {
                self.starts.push(self.frames.len());
            }
            while let Some(frame) = queue.pop() {
                self.frames.push(frame);
            }
        }
        self.queues.retain(|q| Arc::strong_count(q) > 1);
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        registry().enabled.store(false, Ordering::Relaxed);
    }
}
