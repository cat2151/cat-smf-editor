use super::{Command, Part};
use crate::play::instance_cache::Cache;
use crate::play::layer_render::Render;
use crate::{play::audio_sequence::Cursor, play::audition_native::Native};
use cat_smf_editor::score::instrument::Patch;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// A track's plugin with its held-note state, carried into the next stream.
type Reusable = Cache<(Native, Render)>;

pub(super) struct Voice {
    // Declaration order stops the callback before freeing plugins on the UI thread.
    stream: cpal::Stream,
    pub natives: Vec<Native>,
    cached: Reusable,
    pub master: Option<cat_smf_editor::chain::Chain>,
    pub patches: Vec<(usize, Patch)>,
    pub mailbox: Arc<Mutex<Command>>,
    pub rate: u32,
    pub failed: Arc<AtomicBool>,
    // The stopped callback's note state supplies note-offs before plugins are parked.
    renders: Arc<Mutex<Vec<Render>>>,
}
impl Voice {
    pub fn pause(&self) {
        if self.stream.pause().is_err() {
            self.failed.store(true, Ordering::Relaxed);
        }
    }
    pub fn resume(&self) {
        if self.stream.play().is_err() {
            self.failed.store(true, Ordering::Relaxed);
        }
    }
    /// Active and dormant plugins are reused per track, reloading state when needed.
    pub fn open(
        patches: &[(usize, Patch)],
        previous: Option<Self>,
        master_settings: &cat_smf_editor::master_effects::Settings,
        auto_gain_disabled: bool,
    ) -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No audio output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config = supported.config();
        let rate = config.sample_rate.0;
        let (mut reusable, reusable_master) = match previous {
            Some(voice) if voice.rate == rate => voice.into_reusable(),
            _ => (Reusable::default(), None),
        };
        let mut natives = Vec::new();
        let mut renders = Vec::new();
        for (track, patch) in patches {
            let (native, render) = take_or_open(&mut reusable, *track, patch, rate)
                .map_err(|e| format!("Track {}: {e}", track + 1))?;
            natives.push(native);
            renders.push(render);
        }
        let master = if master_settings.effects.is_empty() {
            None
        } else if reusable_master.is_some() {
            reusable_master
        } else {
            Some(
                cat_smf_editor::chain::Chain::open_master(rate, master_settings)
                    .map_err(|e| format!("Master: {e}"))?,
            )
        };
        let master_pointer = master.as_ref().map(|m| unsafe { m.audio_ref() });
        let renders = Arc::new(Mutex::new(renders));
        let mailbox = Arc::new(Mutex::new(Command {
            auto_gain_disabled,
            ..Command::default()
        }));
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(
                &device,
                &config,
                renders.clone(),
                master_pointer,
                mailbox.clone(),
                failed.clone(),
            ),
            cpal::SampleFormat::I16 => build::<i16>(
                &device,
                &config,
                renders.clone(),
                master_pointer,
                mailbox.clone(),
                failed.clone(),
            ),
            cpal::SampleFormat::U16 => build::<u16>(
                &device,
                &config,
                renders.clone(),
                master_pointer,
                mailbox.clone(),
                failed.clone(),
            ),
            format => return Err(format!("Unsupported audio format: {format:?}")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            natives,
            cached: reusable,
            master,
            patches: patches.to_vec(),
            mailbox,
            rate,
            failed,
            renders,
        })
    }

    /// Stops the stream and silences held notes before parking the active plugins.
    fn into_reusable(self) -> (Reusable, Option<cat_smf_editor::chain::Chain>) {
        drop(self.stream);
        let renders = Arc::try_unwrap(self.renders)
            .ok()
            .and_then(|m| m.into_inner().ok())
            .unwrap_or_default();
        if renders.len() != self.natives.len() {
            return (self.cached, self.master);
        }
        let mut reusable = self.cached;
        let active = self
            .patches
            .into_iter()
            .zip(self.natives)
            .zip(renders)
            .map(|(((track, patch), native), render)| (track, patch, native, render));
        for (track, patch, native, mut render) in active {
            let mut silent = [0.0; 2048];
            if !render.process(&[0; 128], &[0; 128], 1, &mut silent, 2) {
                continue;
            }
            // All Sound Off also stops release voices in plugins supporting CC120.
            if !native.silence(&mut silent) {
                continue;
            }
            reusable.put(track, patch, (native, render));
        }
        (reusable, self.master)
    }

    pub fn pump_cached(&self) {
        for (native, _) in self.cached.values() {
            native.pump();
        }
    }
    pub fn clear_cached(&mut self) {
        self.cached = Reusable::default();
    }
    #[cfg(test)]
    pub fn cached_count(&self) -> usize {
        self.cached.values().count()
    }
}

/// Reuses an identical plugin, else reloads the same plugins of this track, else opens anew.
fn take_or_open(
    reusable: &mut Reusable,
    track: usize,
    patch: &Patch,
    rate: u32,
) -> Result<(Native, Render), String> {
    let reloaded = match reusable.take(track, patch) {
        Some(((native, render), false)) => return Ok((native, render)),
        Some(((native, _), true)) => native.reload(rate, patch).ok(),
        None => None,
    };
    let native = match reloaded {
        Some(native) => native,
        None => Native::open(rate, patch)?,
    };
    let render = unsafe { native.render() };
    Ok((native, render))
}

struct State {
    cursor: Cursor,
    velocities: [u8; 128],
    durations: [u64; 128],
}
impl Default for State {
    fn default() -> Self {
        Self {
            cursor: Cursor::default(),
            velocities: [0; 128],
            durations: [0; 128],
        }
    }
}

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    renders: Arc<Mutex<Vec<Render>>>,
    mut master: Option<cat_smf_editor::host::ProcessorRef>,
    mailbox: Arc<Mutex<Command>>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    let rate = config.sample_rate.0;
    let mut scratch = vec![0.0; 1024 * channels];
    let mut mixed = vec![0.0; 1024 * channels];
    let mut states = (0..renders.lock().unwrap().len())
        .map(|_| State::default())
        .collect::<Vec<_>>();
    let mut command = mailbox.lock().unwrap().clone();
    let mut seen = 0;
    let mut elapsed = 0u64;
    let mut auto_gain = crate::play::auto_gain::MasterAutoGain::default();
    let tap = crate::play::analysis::Tap::register(rate);
    let callback_failed = failed.clone();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                if let Ok(latest) = mailbox.try_lock() {
                    command = latest.clone();
                }
                // Only `Voice::into_reusable` locks this too, after this stream is dropped.
                let Ok(mut pointers) = renders.try_lock() else {
                    data.fill(T::EQUILIBRIUM);
                    return;
                };
                let mut offset = 0;
                while offset < data.len() {
                    let fresh = seen != command.generation;
                    seen = command.generation;
                    if fresh {
                        elapsed = 0;
                        for state in &mut states {
                            state.cursor = Cursor::default();
                        }
                    }
                    let max = ((data.len() - offset) / channels).min(1024);
                    let frames = boundary(&states, &command.parts, rate, max);
                    let len = frames * channels;
                    mixed[..len].fill(0.0);
                    for (i, (pointer, state)) in pointers.iter_mut().zip(&mut states).enumerate() {
                        let changed = if let Some(part) = command.parts.get(i) {
                            if let Some(sequence) = &part.sequence {
                                let (_, changed) = state.cursor.block(
                                    sequence,
                                    rate,
                                    frames,
                                    &mut state.velocities,
                                    &mut state.durations,
                                );
                                if fresh {
                                    elapsed = 0;
                                    1
                                } else {
                                    changed
                                }
                            } else {
                                state.velocities = part.velocities;
                                state.durations = part.durations;
                                i32::from(fresh)
                            }
                        } else {
                            state.velocities.fill(0);
                            i32::from(fresh)
                        };
                        // SAFETY: Voice stops this one stream before destroying every Native.
                        if !pointer.process(
                            &state.velocities,
                            &state.durations,
                            changed,
                            &mut scratch[..len],
                            channels,
                        ) {
                            callback_failed.store(true, Ordering::Relaxed);
                            scratch[..len].fill(0.0);
                        }
                        if !mix_into(
                            &mut mixed[..len],
                            &scratch[..len],
                            command.gains.get(i).copied().unwrap_or(1.0),
                        ) {
                            callback_failed.store(true, Ordering::Relaxed);
                        }
                    }
                    if let Some(master) = &mut master
                        && !master.process_input(&mut mixed[..len], channels)
                    {
                        callback_failed.store(true, Ordering::Relaxed);
                        mixed[..len].fill(0.0);
                    }
                    if !mixed[..len].iter().all(|s| s.is_finite()) {
                        callback_failed.store(true, Ordering::Relaxed);
                        mixed[..len].fill(0.0);
                    }
                    auto_gain.process(
                        &mut mixed[..len],
                        rate,
                        channels,
                        !command.auto_gain_disabled,
                    );
                    tap.write(&mixed[..len], channels);
                    for (target, sample) in data[offset..offset + len].iter_mut().zip(&mixed[..len])
                    {
                        *target = T::from_sample(sample.clamp(-1.0, 1.0));
                    }
                    offset += len;
                    elapsed = elapsed.saturating_add(frames as u64);
                    command.progress.store(elapsed, Ordering::Relaxed);
                }
            },
            move |_| {
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())
}

fn mix_into(mixed: &mut [f32], samples: &[f32], gain: f32) -> bool {
    for (sum, sample) in mixed.iter_mut().zip(samples) {
        *sum += sample * gain;
    }
    if !mixed.iter().all(|s| s.is_finite()) {
        mixed.fill(0.0);
        return false;
    }
    true
}

fn boundary(states: &[State], parts: &[Part], rate: u32, max: usize) -> usize {
    states
        .iter()
        .zip(parts)
        .filter_map(|(state, part)| {
            let sequence = part.sequence.as_ref()?;
            let mut cursor = state.cursor.clone();
            let mut velocities = [0; 128];
            let mut durations = [0; 128];
            Some(
                cursor
                    .block(sequence, rate, max, &mut velocities, &mut durations)
                    .0,
            )
        })
        .min()
        .unwrap_or(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::audio_sequence::{Note, Sequence};
    #[test]
    fn volume_scales_audio_and_mute_leaves_other_tracks_unchanged() {
        let mut mixed = [0.0; 2];
        mix_into(&mut mixed, &[0.8, -0.4], 0.25);
        assert_eq!(mixed, [0.2, -0.1]);
        mix_into(&mut mixed, &[0.9, 0.9], 0.0);
        assert_eq!(mixed, [0.2, -0.1]);
        mix_into(&mut mixed, &[0.3, 0.3], 1.0);
        assert_eq!(mixed[0], 0.5);
        assert!((mixed[1] - 0.2).abs() < 1e-6);
        assert!(!mix_into(&mut mixed, &[f32::MAX, f32::MAX], 2.0));
        assert_eq!(mixed, [0.0, 0.0]);
    }
    #[test]
    fn all_tracks_share_frame_boundaries_and_same_pitch_durations_stay_independent() {
        let mut states = [State::default(), State::default()];
        let parts = [0.1, 0.2].map(|start| Part {
            sequence: Some(Arc::new(Sequence {
                notes: vec![Note {
                    start,
                    duration: start,
                    pitch: 60,
                    velocity: 100,
                }],
                end: start * 2.0,
            })),
            ..Part::default()
        });
        for expected_frame in [0, 100, 200, 400] {
            assert_eq!(states[0].cursor.frame, expected_frame);
            assert_eq!(states[1].cursor.frame, expected_frame);
            let frames = boundary(&states, &parts, 1000, 1024);
            let mut changes = Vec::new();
            for (state, part) in states.iter_mut().zip(&parts) {
                let (actual, changed) = state.cursor.block(
                    part.sequence.as_ref().unwrap(),
                    1000,
                    frames,
                    &mut state.velocities,
                    &mut state.durations,
                );
                assert_eq!(actual, frames);
                changes.push(changed);
            }
            if expected_frame == 100 {
                assert_eq!(changes, vec![2, 0]);
                assert_eq!(states[0].durations[60], 100);
            }
            if expected_frame == 200 {
                assert_eq!(changes, vec![3, 2]);
                assert_eq!(states[1].durations[60], 200);
            }
        }
    }
}
