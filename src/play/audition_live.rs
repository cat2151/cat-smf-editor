//! Ordered live notes share the existing layer routing and never restart held voices.
use super::*;
use std::{collections::VecDeque, sync::Mutex};

pub(crate) type Queue = Arc<Mutex<VecDeque<u32>>>;

impl Audition {
    /// Open the instrument before the first key establishes the recording clock.
    pub fn start_live(&mut self, track: usize, patch: &Patch) -> Option<Queue> {
        self.stop_key();
        let disabled = self.disabled;
        let voice = self.key_voice.get_or_insert_with(|| {
            Box::new(Self {
                disabled,
                ..Self::default()
            })
        });
        voice.set_master(self.master.clone());
        voice.gains = self.gains.clone();
        voice.set_auto_gain_enabled(!self.auto_gain_disabled);
        if !voice.prepare(&[(track, patch.clone())]) {
            return None;
        }
        let queue = Queue::default();
        voice.send(vec![(
            track,
            Part {
                live: Some(queue.clone()),
                ..Part::default()
            },
        )]);
        self.key_active = true;
        Some(queue)
    }
}

pub(crate) fn push(queue: &Queue, pitch: u8, on: bool) {
    queue
        .lock()
        .unwrap()
        .push_back((if on { 0x2090_0050 } else { 0x2080_0000 }) | (u32::from(pitch) << 8));
}

#[cfg(test)]
#[path = "audition_live_tests.rs"]
mod tests;
