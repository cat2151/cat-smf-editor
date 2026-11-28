//! TONE3000 reads its model after it opens and passes no sound until it has; its first
//! ~40ms of processed sound then comes out quiet (ADR 0025). Before a song, a fresh
//! instance waits until it answers a test tone, then runs silence through the whole track.
use super::Native;
use crate::play::layer_render::Render;
use cat_smf_editor::host::ProcessorRef;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const TONE3000: &str = "com.tone3000.plugin";
const BLOCK: usize = 1024;
const CHANNELS: usize = 2;
/// Same floor as the render tests use to find where a part starts to sound.
const AUDIBLE: f32 = 1e-3;
/// Left-over sound below this cannot move where a song seems to start.
const QUIET: f32 = 1e-5;
/// Twice the processed sound the quiet start was measured to last.
const WARM_FRAMES: usize = 4096;
/// Past this the model is taken as never loading and the song goes on as before.
const LIMIT: Duration = Duration::from_secs(3);
const RETRY: Duration = Duration::from_millis(5);

pub struct Settle {
    effects: ProcessorRef,
}

impl Native {
    /// None unless the track's effects need settling before a song.
    ///
    /// # Safety
    /// The settle must run before any stream renders this Native, and finish before it drops.
    pub unsafe fn settle(&self) -> Option<Settle> {
        let effects = self.effects.as_ref()?;
        self.settings
            .effects
            .iter()
            .any(|effect| effect.plugin.id == TONE3000 && !effect.bypass)
            .then(|| Settle {
                effects: unsafe { effects.audio_ref() },
            })
    }
}

impl Settle {
    /// `render` must be this Native's. False once `cancel` is set.
    pub fn run(mut self, render: &mut Render, rate: u32, cancel: &AtomicBool) -> bool {
        let started = Instant::now();
        let mut out = [0.0f32; BLOCK * CHANNELS];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            tone(&mut out, rate);
            let processed = self.effects.process_input(&mut out, CHANNELS);
            if !processed || peak(&out) > AUDIBLE || started.elapsed() >= LIMIT {
                break;
            }
            std::thread::sleep(RETRY);
        }
        // The test tone's tail dies away here as well.
        let mut frames = 0;
        while frames < rate as usize * 2 {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            if !render.process(&[0; 128], &[0; 128], 0, &mut out, CHANNELS) {
                break;
            }
            frames += BLOCK;
            if frames >= WARM_FRAMES && peak(&out) < QUIET {
                break;
            }
        }
        true
    }
}

/// A 440Hz sine at half scale on both channels.
fn tone(out: &mut [f32], rate: u32) {
    for (i, frame) in out.chunks_mut(CHANNELS).enumerate() {
        let v = 0.5 * (std::f32::consts::TAU * 440.0 * i as f32 / rate as f32).sin();
        frame.fill(v);
    }
}

fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0, |m, s| m.max(s.abs()))
}
