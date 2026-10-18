//! Faster-than-realtime rendering into memory, through the playback mixer, without a device.
use super::{mix::Mixer, Command, Part};
use crate::play::{audio_sequence::Sequence, audition_native::Native, layer_render::Render};
use cat_smf_editor::{chain::Chain, master_effects, score::instrument::Patch};
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};

pub const CHANNELS: usize = 2;

/// Plugins of one render. They stay on the UI thread, which must keep pumping them.
pub struct Instruments {
    natives: Vec<Native>,
    master: Option<Chain>,
}

/// The audio-thread half of [`Instruments`], movable to a worker thread.
pub struct Job {
    renders: Vec<Render>,
    master: Option<cat_smf_editor::host::ProcessorRef>,
    command: Command,
    rate: u32,
    frames: u64,
}

impl Instruments {
    /// Tracks without notes only span time and open no plugin.
    pub fn open(
        rate: u32,
        parts: &[(usize, Patch, Sequence)],
        master: &master_effects::Settings,
    ) -> Result<Self, String> {
        let natives = parts
            .iter()
            .filter(|(_, _, sequence)| !sequence.notes.is_empty())
            .map(|(track, patch, _)| {
                Native::open(rate, patch).map_err(|e| format!("Track {}: {e}", track + 1))
            })
            .collect::<Result<_, _>>()?;
        let master = if master.effects.is_empty() {
            None
        } else {
            Some(Chain::open_master(rate, master).map_err(|e| format!("Master: {e}"))?)
        };
        Ok(Self { natives, master })
    }

    pub fn pump(&self) {
        for native in &self.natives {
            native.pump();
        }
        if let Some(master) = &self.master {
            master.pump();
        }
    }

    /// `parts` and `gains` must be those given to [`Self::open`].
    ///
    /// # Safety
    /// The returned job must finish before these instruments are dropped.
    pub unsafe fn job(
        &self,
        rate: u32,
        parts: Vec<(usize, Patch, Sequence)>,
        gains: &[f32],
        auto_gain_disabled: bool,
    ) -> Job {
        let end = parts.iter().map(|(_, _, s)| s.end).fold(0.0, f64::max);
        let (parts, gains): (Vec<_>, Vec<_>) = parts
            .into_iter()
            .filter(|(_, _, sequence)| !sequence.notes.is_empty())
            .map(|(track, _, sequence)| {
                (
                    Part {
                        sequence: Some(Arc::new(sequence)),
                        ..Part::default()
                    },
                    gains.get(track).copied().unwrap_or(1.0),
                )
            })
            .unzip();
        Job {
            renders: self.natives.iter().map(|n| unsafe { n.render() }).collect(),
            master: self.master.as_ref().map(|m| unsafe { m.audio_ref() }),
            command: Command {
                generation: 1,
                parts: Arc::new(parts),
                gains: Arc::new(gains),
                auto_gain_disabled,
                ..Command::default()
            },
            rate,
            frames: (end * f64::from(rate)).round().max(0.0) as u64,
        }
    }
}

impl Job {
    /// Interleaved stereo; None once `cancel` is set.
    pub fn run(mut self, cancel: &AtomicBool) -> Option<Result<Vec<f32>, String>> {
        let failed = AtomicBool::new(false);
        let mut mixer = Mixer::new(self.rate, CHANNELS, self.renders.len(), self.master);
        let mut out = Vec::with_capacity(self.frames as usize * CHANNELS);
        while mixer.elapsed < self.frames {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let max = (self.frames - mixer.elapsed).min(1024) as usize;
            out.extend_from_slice(mixer.block(&mut self.renders, &self.command, max, &failed));
            if failed.load(Ordering::Relaxed) {
                return Some(Err("Audio: plugin stopped while rendering".into()));
            }
        }
        fade_out(&mut out, self.rate);
        Some(Ok(out))
    }
}

/// The measure ends mid-sound; a short fade keeps the cut from clicking.
fn fade_out(samples: &mut [f32], rate: u32) {
    let frames = (samples.len() / CHANNELS).min(rate as usize / 100);
    let start = samples.len() - frames * CHANNELS;
    for (i, frame) in samples[start..].chunks_mut(CHANNELS).enumerate() {
        let gain = 1.0 - (i + 1) as f32 / frames as f32;
        for sample in frame {
            *sample *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fade_reaches_silence_at_the_end_and_leaves_the_head() {
        let mut samples = vec![1.0; 2 * 1000];
        super::fade_out(&mut samples, 1000);
        assert_eq!(samples[0], 1.0);
        assert_eq!(samples[2 * 989], 1.0);
        assert!(samples[2 * 995] < 1.0);
        assert_eq!(samples[2 * 999..], [0.0, 0.0]);
    }
}
