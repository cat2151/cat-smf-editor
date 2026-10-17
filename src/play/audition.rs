//! Track-local plugin instances, synchronized through one output stream.
use crate::play::audio_sequence::Sequence;
use cat_smf_editor::score::instrument::Patch;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
#[path = "audition_output.rs"]
mod output;
use output::Voice;

pub(super) struct Part {
    velocities: [u8; 128],
    durations: [u64; 128],
    sequence: Option<Arc<Sequence>>,
}
impl Default for Part {
    fn default() -> Self {
        Self {
            velocities: [0; 128],
            durations: [0; 128],
            sequence: None,
        }
    }
}
#[derive(Clone, Default)]
pub(super) struct Command {
    generation: u64,
    parts: Arc<Vec<Part>>,
    progress: Arc<AtomicU64>,
    gains: Arc<Vec<f32>>,
    auto_gain_disabled: bool,
}
#[derive(Default)]
pub struct Audition {
    /// Independent plugin instances for live keys, leaving sequence playback intact.
    key_voice: Option<Box<Audition>>,
    key_active: bool,
    voice: Option<Voice>,
    error: Option<String>,
    disabled: bool,
    prepared: bool,
    progress: Option<Arc<AtomicU64>>,
    gains: std::collections::HashMap<usize, f32>,
    auto_gain_disabled: bool,
    master: cat_smf_editor::master_effects::Settings,
    last_browser_use: Option<std::time::Instant>,
    /// Pitches of the last `play_track`, observable without an audio device.
    #[cfg(test)]
    pub auditioned: Vec<u8>,
}
impl Audition {
    /// Move prepared audio into a session cache without carrying document gains.
    pub fn take_prepared(&mut self) -> Option<Self> {
        if !self.is_prepared() {
            return None;
        }
        self.stop();
        if let Some(voice) = &self.voice {
            voice.pause();
        }
        Some(Self {
            voice: self.voice.take(),
            master: self.master.clone(),
            auto_gain_disabled: self.auto_gain_disabled,
            disabled: self.disabled,
            prepared: true,
            ..Self::default()
        })
    }
    pub fn reuse_prepared(&mut self, mut cached: Self) {
        if self.master == cached.master && cached.is_prepared() {
            self.voice = cached.voice.take();
            self.prepared = cached.prepared;
            if let Some(voice) = &self.voice {
                voice.mailbox.lock().unwrap().auto_gain_disabled = self.auto_gain_disabled;
                voice.resume();
            }
        }
    }
    pub fn is_prepared(&self) -> bool {
        self.voice
            .as_ref()
            .is_some_and(|v| !v.failed.load(Ordering::Relaxed))
            || self.disabled && self.prepared
    }
    pub fn master(&self) -> &cat_smf_editor::master_effects::Settings {
        &self.master
    }
    pub fn set_master(&mut self, master: cat_smf_editor::master_effects::Settings) {
        if self.master == master {
            return;
        }
        let live = cat_smf_editor::score::layer_track::same_except_bypass(
            &self.master.effects,
            &master.effects,
        ) && self.voice.as_ref().is_none_or(|voice| {
            voice
                .master
                .as_ref()
                .is_some_and(|chain| chain.set_master_bypass(&master.effects).is_ok())
        });
        if !live {
            self.reset();
        }
        self.master = master;
    }
    /// Applies a bypass-only change of `track` to the running voices.
    pub fn set_effect_bypass(&mut self, track: usize, patch: &Patch) {
        if let Some(voice) = &mut self.key_voice {
            voice.set_effect_bypass(track, patch);
        }
        let Some(voice) = &mut self.voice else {
            return;
        };
        let Some(index) = voice.patches.iter().position(|(t, _)| *t == track) else {
            return;
        };
        match voice.natives[index].set_bypass(patch) {
            Ok(()) => voice.patches[index].1 = patch.clone(),
            Err(error) => {
                self.reset();
                self.error = Some(format!("Audio: {error}"));
            }
        }
    }
    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            disabled: true,
            ..Self::default()
        }
    }
    pub fn error(&self) -> Option<&str> {
        self.error
            .as_deref()
            .or_else(|| self.key_voice.as_ref().and_then(|v| v.error()))
    }
    pub fn needs_pump(&self) -> bool {
        self.voice.is_some() || self.key_voice.as_ref().is_some_and(|v| v.needs_pump())
    }
    pub fn reset(&mut self) {
        self.key_voice = None;
        self.key_active = false;
        self.voice = None;
        self.prepared = false;
        self.progress = None;
        self.error = None;
    }
    fn prepare(&mut self, patches: &[(usize, Patch)]) -> bool {
        if self.disabled {
            self.prepared = true;
            return true;
        }
        self.last_browser_use
            .get_or_insert_with(std::time::Instant::now);
        // Tracks keep their plugins; only a new track or a changed patch opens one.
        let mut wanted = self
            .voice
            .as_ref()
            .map_or_else(Vec::new, |v| v.patches.clone());
        let mut missing = self.voice.is_none();
        for (track, patch) in patches {
            match wanted.iter_mut().find(|(t, _)| t == track) {
                Some((_, p)) if p == patch => {}
                Some((_, p)) => {
                    *p = patch.clone();
                    missing = true;
                }
                None => {
                    wanted.push((*track, patch.clone()));
                    missing = true;
                }
            }
        }
        if !missing {
            return true;
        }
        let previous = self.voice.take();
        self.progress = None;
        self.error = None;
        match Voice::open(&wanted, previous, &self.master, self.auto_gain_disabled) {
            Ok(voice) => {
                self.voice = Some(voice);
                true
            }
            Err(error) => {
                self.error = Some(format!("Audio: {error}"));
                false
            }
        }
    }
    /// Tracks of the voice without a part fall silent.
    fn send(&mut self, mut parts: Vec<(usize, Part)>) {
        let progress = Arc::new(AtomicU64::new(0));
        self.progress = parts
            .iter()
            .any(|(_, p)| p.sequence.is_some())
            .then(|| progress.clone());
        if let Some(voice) = &self.voice {
            let parts = voice
                .patches
                .iter()
                .map(|(track, _)| {
                    parts
                        .iter()
                        .position(|(t, _)| t == track)
                        .map_or_else(Part::default, |i| parts.swap_remove(i).1)
                })
                .collect();
            let mut command = voice.mailbox.lock().unwrap();
            command.generation = command.generation.wrapping_add(1);
            command.parts = Arc::new(parts);
            command.progress = progress;
            command.gains = Arc::new(
                voice
                    .patches
                    .iter()
                    .map(|(track, _)| self.gain(*track))
                    .collect(),
            );
        }
    }
    pub fn play_track(
        &mut self,
        track: usize,
        patch: &Patch,
        notes: &[(u8, u8, Option<f64>)],
    ) -> bool {
        if !self.prepare(&[(track, patch.clone())]) {
            return false;
        }
        #[cfg(test)]
        {
            self.auditioned = notes.iter().map(|&(pitch, _, _)| pitch).collect();
        }
        let rate = self.voice.as_ref().map_or(48000, |v| v.rate);
        let mut part = Part::default();
        for &(pitch, velocity, seconds) in notes {
            let i = usize::from(pitch);
            if i >= 128 {
                continue;
            }
            part.velocities[i] = part.velocities[i].max(velocity);
            part.durations[i] = part.durations[i]
                .max(seconds.map_or(u64::MAX, |s| (s * f64::from(rate)).round().max(0.0) as u64));
        }
        self.send(vec![(track, part)]);
        true
    }
    pub fn sequences(&mut self, parts: Vec<(usize, Patch, Sequence)>) -> bool {
        if parts.is_empty() {
            self.stop();
            return true;
        }
        // A track without notes, such as a conductor track, only spans time.
        let patches = parts
            .iter()
            .filter(|(_, _, sequence)| !sequence.notes.is_empty())
            .map(|(track, patch, _)| (*track, patch.clone()))
            .collect::<Vec<_>>();
        if !self.prepare(&patches) {
            return false;
        }
        self.send(
            parts
                .into_iter()
                .map(|(track, _, sequence)| {
                    (
                        track,
                        Part {
                            sequence: Some(Arc::new(sequence)),
                            ..Part::default()
                        },
                    )
                })
                .collect(),
        );
        true
    }
    #[cfg(test)]
    pub fn set_test_seconds(&self, seconds: f64) {
        self.progress
            .as_ref()
            .unwrap()
            .store((seconds * 48000.0).round() as u64, Ordering::Relaxed);
    }
    pub fn seconds(&self) -> Option<f64> {
        self.progress.as_ref().map(|p| {
            p.load(Ordering::Relaxed) as f64
                / f64::from(self.voice.as_ref().map_or(48000, |v| v.rate))
        })
    }
    pub fn gain(&self, track: usize) -> f32 {
        self.gains.get(&track).copied().unwrap_or(1.0)
    }
    pub fn auto_gain_enabled(&self) -> bool {
        !self.auto_gain_disabled
    }
    pub fn set_auto_gain_enabled(&mut self, enabled: bool) {
        self.auto_gain_disabled = !enabled;
        if let Some(voice) = &mut self.key_voice {
            voice.set_auto_gain_enabled(enabled);
        }
        if let Some(voice) = &self.voice {
            voice.mailbox.lock().unwrap().auto_gain_disabled = !enabled;
        }
    }
    pub fn set_gain(&mut self, track: usize, gain: f32) {
        if let Some(voice) = &mut self.key_voice {
            voice.set_gain(track, gain);
        }
        self.gains.insert(
            track,
            gain.clamp(
                0.0,
                crate::play::track_volume::db_to_gain(crate::play::track_volume::MAX_DB),
            ),
        );
        if let Some(voice) = &self.voice {
            voice.mailbox.lock().unwrap().gains =
                Arc::new(voice.patches.iter().map(|(t, _)| self.gain(*t)).collect());
        }
    }
    pub fn clear_gains(&mut self) {
        self.gains.clear();
        if let Some(voice) = &mut self.key_voice {
            voice.clear_gains();
        }
    }
    pub fn stop(&mut self) {
        self.stop_key();
        self.send(Vec::new());
    }
    pub fn browser_used(&mut self) {
        self.last_browser_use = Some(std::time::Instant::now());
    }
    pub fn pump(&mut self) {
        if let Some(voice) = &mut self.key_voice {
            voice.pump();
        }
        self.expire_cached(std::time::Instant::now());
        if let Some(voice) = &self.voice {
            voice.pump_cached();
            for native in &voice.natives {
                native.pump();
            }
            if let Some(master) = &voice.master {
                master.pump();
            }
        }
        if let Some(voice) = &self.voice
            && (voice.failed.load(Ordering::Relaxed))
        {
            self.voice = None;
            self.progress = None;
            self.error = Some("Audio: plugin or audio device stopped".into());
        }
    }
    fn expire_cached(&mut self, now: std::time::Instant) {
        if self.last_browser_use.is_some_and(|last| {
            now.saturating_duration_since(last) >= std::time::Duration::from_secs(5 * 60)
        }) && let Some(voice) = &mut self.voice
        {
            voice.clear_cached();
        }
    }
}

#[path = "audition_key.rs"]
mod key;

#[cfg(test)]
#[path = "audition_cache_tests.rs"]
mod cache_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn master_auto_gain_defaults_on_and_selection_resets_preserve_the_switch() {
        let mut audition = Audition::disabled();
        assert!(audition.auto_gain_enabled());
        audition.set_auto_gain_enabled(false);
        audition.reset();
        audition.stop();
        assert!(!audition.auto_gain_enabled());
        audition.set_auto_gain_enabled(true);
        assert!(audition.auto_gain_enabled());
    }
    #[test]
    #[ignore = "requires installed Surge XT and default audio output; emits audio"]
    fn default_device_plays_two_patches_in_one_stream() {
        let mut audition = Audition::default();
        let ep = Patch::default();
        let sequence = |duration| Sequence {
            notes: vec![crate::play::audio_sequence::Note {
                start: 0.05,
                duration,
                pitch: 60,
                velocity: 80,
            }],
            end: duration + 0.05,
        };
        assert!(
            audition.sequences(vec![
                (0, Patch::default(), sequence(0.25)),
                (1, ep.clone(), sequence(0.5))
            ]),
            "{:?}",
            audition.error()
        );
        assert_eq!(audition.voice.as_ref().unwrap().natives.len(), 2);
        assert_eq!(audition.voice.as_ref().unwrap().patches[1].1, ep);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(650);
        while std::time::Instant::now() < deadline {
            audition.pump();
            assert!(audition.error().is_none(), "{:?}", audition.error());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        audition.stop();
    }
    #[test]
    #[ignore = "requires installed Surge XT and default audio output; emits audio"]
    fn default_device_auditions_duration_and_can_stop() {
        let mut audition = Audition::default();
        assert!(
            audition.play_track(
                0,
                &Patch::default(),
                &[(60, 80, Some(0.25)), (67, 70, Some(0.5))]
            ),
            "{:?}",
            audition.error()
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(650);
        while std::time::Instant::now() < deadline {
            audition.pump();
            assert!(audition.error().is_none(), "{:?}", audition.error());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        audition.stop();
    }
}

#[cfg(test)]
#[path = "audition_session_tests.rs"]
mod session_tests;
