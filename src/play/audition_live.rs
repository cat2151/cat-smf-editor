//! Ordered live notes share the existing layer routing and never restart held voices.
use super::*;
use std::{collections::VecDeque, sync::Mutex};

pub(crate) type Queue = Arc<Mutex<VecDeque<u32>>>;

/// What the keyboard shares with the audio thread.
pub(crate) struct Live {
    pub queue: Queue,
    pub click: Arc<super::click::Click>,
    pub phrase: Arc<super::phrase::Phrase>,
}

impl Audition {
    /// Open the instrument before the first key establishes the recording clock.
    pub fn start_live(&mut self, track: usize, patch: &Patch) -> Option<Live> {
        self.stop_live();
        let live = Live {
            queue: Queue::default(),
            click: Arc::new(super::click::Click::new()),
            phrase: Arc::new(super::phrase::Phrase::default()),
        };
        self.retune_live(track, patch, &live).then_some(live)
    }

    /// Plays `live` with `patch`; queued keys, the metronome and the phrase keep their time.
    pub fn retune_live(&mut self, track: usize, patch: &Patch, live: &Live) -> bool {
        let disabled = self.disabled;
        let voice = self.live_voice.get_or_insert_with(|| {
            Box::new(Self {
                disabled,
                ..Self::default()
            })
        });
        voice.set_master(self.master.clone());
        voice.gains = self.gains.clone();
        voice.set_auto_gain_enabled(!self.auto_gain_disabled);
        if !voice.prepare(&[(track, patch.clone())]) {
            return false;
        }
        voice.send(vec![(
            track,
            Part {
                live: Some(live.queue.clone()),
                click: Some(live.click.clone()),
                phrase: Some(live.phrase.clone()),
                ..Part::default()
            },
        )]);
        #[cfg(test)]
        {
            self.live_active = true;
            self.live_patch = Some(patch.clone());
        }
        true
    }

    #[cfg(test)]
    pub fn live_active(&self) -> bool {
        self.live_active
    }

    /// The voice keeps its plugins for the next keyboard.
    pub fn stop_live(&mut self) {
        #[cfg(test)]
        {
            self.live_active = false;
        }
        if let Some(voice) = &mut self.live_voice {
            voice.stop();
        }
    }
}

pub(crate) fn push(queue: &Queue, pitch: u8, on: bool) {
    queue.lock().unwrap().push_back(word(pitch, on));
}

pub(crate) fn word(pitch: u8, on: bool) -> u32 {
    (if on { 0x2090_0050 } else { 0x2080_0000 }) | (u32::from(pitch) << 8)
}

#[cfg(test)]
#[path = "audition_live_tests.rs"]
mod tests;
