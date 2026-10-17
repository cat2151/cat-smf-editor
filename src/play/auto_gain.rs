//! Master adaptation of clap-mml-play-server's player/auto_gain.rs used by
//! clap-mml-render-tui: -12 dBFS RMS, -60 dBFS gate, -12..+6 dB trim,
//! 120 dB/s attenuation and 6 dB/s boost, with no added audio latency.

#[derive(Default)]
pub struct MasterAutoGain {
    gain_db: f32,
    measured: bool,
}

impl MasterAutoGain {
    pub fn process(&mut self, samples: &mut [f32], rate: u32, channels: usize, enabled: bool) {
        if !enabled {
            *self = Self::default();
            return;
        }
        if samples.is_empty() || channels == 0 || rate == 0 {
            return;
        }
        let start = amplitude(self.gain_db);
        let power = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
        if power.is_finite() && power >= 1.0e-6 {
            let desired = (-12.0 - 10.0 * power.log10()).clamp(-12.0, 6.0);
            self.gain_db = if self.measured {
                let speed = if desired < self.gain_db { 120.0 } else { 6.0 };
                let seconds = (samples.len() / channels) as f32 / rate as f32;
                let limit = speed * seconds;
                self.gain_db + (desired - self.gain_db).clamp(-limit, limit)
            } else {
                self.measured = true;
                desired.min(0.0)
            };
        }
        let end = amplitude(self.gain_db);
        let frames = samples.len().div_ceil(channels);
        for (frame, values) in samples.chunks_mut(channels).enumerate() {
            let fraction = (frame + 1) as f32 / frames as f32;
            let gain = start + (end - start) * fraction;
            for sample in values {
                *sample *= gain;
            }
        }
    }
}

fn amplitude(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_audio_rises_slowly_to_six_db_and_off_restores_unity() {
        let mut gain = MasterAutoGain::default();
        let mut first = [0.01; 960];
        gain.process(&mut first, 48_000, 2, true);
        assert_eq!(first, [0.01; 960]);
        let mut second = [0.01; 960];
        gain.process(&mut second, 48_000, 2, true);
        assert!(second[959] > 0.01 && second[959] < 0.0101);
        for _ in 0..200 {
            gain.process(&mut [0.01; 960], 48_000, 2, true);
        }
        assert!((gain.gain_db - 6.0).abs() < 1e-5);
        let mut dry = [0.01; 960];
        gain.process(&mut dry, 48_000, 2, false);
        assert_eq!(dry, [0.01; 960]);
        assert_eq!(gain.gain_db, 0.0);
        assert!(!gain.measured);
    }

    #[test]
    fn loud_mix_is_attenuated_with_a_stereo_linked_ramp_and_silence_holds_trim() {
        let mut gain = MasterAutoGain::default();
        let mut loud = [1.0; 960];
        gain.process(&mut loud, 48_000, 2, true);
        assert_eq!(gain.gain_db, -12.0);
        assert!(loud[0] > loud[958]);
        assert!((loud[959] - amplitude(-12.0)).abs() < 1e-6);
        for frame in loud.chunks(2) {
            assert_eq!(frame[0], frame[1]);
        }
        let mut silence = [0.0; 960];
        gain.process(&mut silence, 48_000, 2, true);
        assert_eq!(silence, [0.0; 960]);
        assert_eq!(gain.gain_db, -12.0);
        gain.process(&mut [0.0001; 960], 48_000, 2, true);
        assert_eq!(gain.gain_db, -12.0);
    }

    #[test]
    fn slew_depends_on_frames_rather_than_channel_count() {
        let mut mono = MasterAutoGain::default();
        let mut stereo = MasterAutoGain::default();
        for _ in 0..10 {
            mono.process(&mut [0.01; 480], 48_000, 1, true);
            stereo.process(&mut [0.01; 960], 48_000, 2, true);
        }
        assert!((mono.gain_db - stereo.gain_db).abs() < 1e-6);
    }
}
