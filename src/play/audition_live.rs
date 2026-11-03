//! Ordered live notes share the existing layer routing and never restart held voices.
use super::*;
use cat_smf_editor::score::pattern::recording::Modulation;
use std::{collections::VecDeque, sync::Mutex, time::Instant};

pub(crate) type Queue = Arc<Mutex<Keys>>;

/// The keyboard's MIDI: queued key words and CC1 on the keyboard session's clock.
pub(crate) struct Keys {
    pub words: VecDeque<u32>,
    /// The keyboard session's clock, whose zero the CC1 sweep starts from.
    clock: Instant,
    modulation: Modulation,
    /// The CC1 value last handed to the audio thread.
    cc1: Option<u8>,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            words: VecDeque::new(),
            clock: Instant::now(),
            modulation: Modulation::Off,
            cc1: None,
        }
    }
}

impl Keys {
    /// Plays `modulation` on the session clock; another choice is sent at once.
    pub(crate) fn modulate(&mut self, clock: Instant, modulation: Modulation) {
        if (self.clock, self.modulation) != (clock, modulation) {
            (self.clock, self.modulation, self.cc1) = (clock, modulation, None);
        }
    }

    /// Writes a changed CC1 value, then the queued words, into `out`; returns how many.
    pub(crate) fn take(&mut self, now: Instant, out: &mut [u32]) -> usize {
        let mut count = 0;
        let cc1 = self
            .modulation
            .value(now.saturating_duration_since(self.clock));
        if let Some(value) = cc1.filter(|_| self.cc1 != cc1 && !out.is_empty()) {
            out[0] = 0x20B0_0100 | u32::from(value);
            self.cc1 = cc1;
            count = 1;
        }
        let queued = self.words.len().min(out.len() - count);
        for (word, queued) in out[count..].iter_mut().zip(self.words.drain(..queued)) {
            *word = queued;
        }
        count + queued
    }
}

/// What the keyboard shares with the audio thread for one container: its keys and the phrase
/// it repeats.
#[derive(Clone, Default)]
pub(crate) struct Live {
    pub queue: Queue,
    pub phrase: Arc<super::phrase::Phrase>,
}

impl Audition {
    /// Open the instruments before the first key establishes the recording clock.
    pub fn start_live(
        &mut self,
        sounds: &[(usize, Patch, Live)],
        click: &Arc<super::click::Click>,
    ) -> bool {
        self.stop_live();
        self.retune_live(sounds, click)
    }

    /// Plays every container of `sounds` at once, the metronome with the first; queued keys,
    /// the metronome and the phrases keep their time.
    pub fn retune_live(
        &mut self,
        sounds: &[(usize, Patch, Live)],
        click: &Arc<super::click::Click>,
    ) -> bool {
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
        let patches: Vec<_> = sounds.iter().map(|(t, p, _)| (*t, p.clone())).collect();
        if !voice.prepare(&patches) {
            return false;
        }
        voice.send(
            sounds
                .iter()
                .enumerate()
                .map(|(i, (track, _, live))| {
                    (
                        *track,
                        Part {
                            live: Some(live.queue.clone()),
                            click: (i == 0).then(|| click.clone()),
                            phrase: Some(live.phrase.clone()),
                            ..Part::default()
                        },
                    )
                })
                .collect(),
        );
        #[cfg(test)]
        {
            self.live_active = true;
            self.live_patches = patches;
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

/// Notes of `pitches` at `velocity`, or released for 0.
pub(crate) fn push(queue: &Queue, pitches: &[u8], velocity: u8) {
    let mut keys = queue.lock().unwrap();
    for &pitch in pitches {
        keys.words.push_back(word(pitch, velocity));
    }
}

/// Note On of `velocity`, or Note Off for 0.
pub(crate) fn word(pitch: u8, velocity: u8) -> u32 {
    (if velocity > 0 {
        0x2090_0000
    } else {
        0x2080_0000
    }) | (u32::from(pitch) << 8)
        | u32::from(velocity & 0x7f)
}

#[cfg(test)]
#[path = "audition_live_tests.rs"]
mod tests;
