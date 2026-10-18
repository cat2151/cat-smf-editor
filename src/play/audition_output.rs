use super::Command;
use crate::play::audition_native::Native;
use crate::play::instance_cache::Cache;
use crate::play::layer_render::Render;
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

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    renders: Arc<Mutex<Vec<Render>>>,
    master: Option<cat_smf_editor::host::ProcessorRef>,
    mailbox: Arc<Mutex<Command>>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    let rate = config.sample_rate.0;
    let tracks = renders.lock().unwrap().len();
    let mut mixer = super::mix::Mixer::new(rate, channels, tracks, master);
    let mut command = mailbox.lock().unwrap().clone();
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
                    let max = (data.len() - offset) / channels;
                    // SAFETY: Voice stops this one stream before destroying every Native.
                    let mixed = mixer.block(&mut pointers, &command, max, &callback_failed);
                    tap.write(mixed, channels);
                    for (target, sample) in data[offset..].iter_mut().zip(mixed) {
                        *target = T::from_sample(sample.clamp(-1.0, 1.0));
                    }
                    offset += mixed.len();
                    command.progress.store(mixer.elapsed, Ordering::Relaxed);
                }
            },
            move |_| {
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())
}
