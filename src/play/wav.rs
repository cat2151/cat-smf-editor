//! Stereo 32-bit float WAV files and their one-shot playback.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{io::Write, path::Path};

const CHANNELS: u16 = 2;

/// Replaces `path` atomically, so a reader never sees a partial file.
pub fn write(path: &Path, rate: u32, samples: &[f32]) -> Result<(), String> {
    let directory = path.parent().ok_or("WAV path has no directory")?;
    std::fs::create_dir_all(directory).map_err(|e| format!("{}: {e}", directory.display()))?;
    let data = (samples.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(CHANNELS) * 4).to_le_bytes());
    bytes.extend_from_slice(&(CHANNELS * 4).to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads a file written by [`write`]: sample rate and interleaved stereo samples.
pub fn read(path: &Path) -> Result<(u32, Vec<f32>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let invalid = || format!("{}: unsupported WAV", path.display());
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err(invalid());
    }
    let u16_at = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut at = 12;
    let mut rate = None;
    while let (Some(id), Some(size)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at + 8;
        let size = size as usize;
        match id {
            b"fmt " => {
                if u16_at(body) != Some(3) || u16_at(body + 2) != Some(CHANNELS) {
                    return Err(invalid());
                }
                rate = u32_at(body + 4);
            }
            b"data" => {
                let data = bytes.get(body..body + size).ok_or_else(invalid)?;
                let samples = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect();
                return Ok((rate.ok_or_else(invalid)?, samples));
            }
            _ => {}
        }
        at = body + size + size % 2;
    }
    Err(invalid())
}

/// Plays once on the default output, resampled to the device rate; stops when dropped.
pub struct Player {
    _stream: cpal::Stream,
}

impl Player {
    pub fn play(rate: u32, samples: Vec<f32>) -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config = supported.config();
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, rate, samples),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, rate, samples),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, rate, samples),
            format => return Err(format!("Unsupported audio format: {format:?}")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { _stream: stream })
    }
}

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rate: u32,
    samples: Vec<f32>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    let step = f64::from(rate) / f64::from(config.sample_rate.0);
    let frames = samples.len() / usize::from(CHANNELS);
    let mut position = 0.0f64;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let (stereo, next) = resampled(&samples, frames, position, step);
                    position = next;
                    for (channel, target) in frame.iter_mut().enumerate() {
                        let sample = stereo.get(channel).copied().unwrap_or(0.0);
                        *target = T::from_sample(sample.clamp(-1.0, 1.0));
                    }
                }
            },
            |_| {},
            None,
        )
        .map_err(|e| e.to_string())
}

/// Linear interpolation at `position` (in source frames), then silence past the end.
fn resampled(samples: &[f32], frames: usize, position: f64, step: f64) -> ([f32; 2], f64) {
    let index = position as usize;
    if index + 1 >= frames {
        return ([0.0; 2], position);
    }
    let t = (position - index as f64) as f32;
    let at = |frame: usize, channel: usize| samples[frame * 2 + channel];
    let mix = |c| at(index, c) + (at(index + 1, c) - at(index, c)) * t;
    ([mix(0), mix(1)], position + step)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn written_samples_read_back_with_rate() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested").join("a.wav");
        write(&path, 44100, &[0.5, -0.25, 1.0, 0.0]).unwrap();
        assert_eq!(read(&path).unwrap(), (44100, vec![0.5, -0.25, 1.0, 0.0]));
        assert_eq!(std::fs::read(&path).unwrap().len(), 44 + 16);
        std::fs::write(&path, b"RIFF....WAVE").unwrap();
        assert!(read(&path).is_err());
    }
    #[test]
    fn resampling_interpolates_and_ends_silent() {
        let samples = [0.0, 1.0, 1.0, 0.0];
        let (mid, next) = resampled(&samples, 2, 0.5, 0.5);
        assert_eq!((mid, next), ([0.5, 0.5], 1.0));
        assert_eq!(resampled(&samples, 2, next, 0.5), ([0.0; 2], 1.0));
    }
    #[test]
    #[ignore = "requires default audio output; emits a short tone"]
    fn default_device_plays_a_rate_converted_file() {
        let samples = (0..22050)
            .flat_map(|i| [(i as f32 * 0.06).sin() * 0.1; 2])
            .collect();
        let player = Player::play(22050, samples).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(player);
    }
}
