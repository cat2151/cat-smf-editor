//! One mixing step shared by the output stream and offline rendering.
use super::{Command, Part};
use crate::play::audio_diagnostics::Counters;
use crate::play::audio_sequence::Cursor;
use crate::play::layer_render::{Effect, Instrument, Render};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
#[path = "audition_mix/batching_tests.rs"]
mod batching_tests;
#[path = "audition_mix/pool.rs"]
mod pool;
#[cfg(test)]
#[path = "audition_mix/pool_tests.rs"]
mod pool_tests;
#[path = "audition_mix/rest.rs"]
mod rest;
pub(crate) use rest::Resting;
#[cfg(test)]
#[path = "audition_mix/rest_tests.rs"]
mod rest_tests;

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

/// Everything one part mutates during a block, so parts never share a buffer.
struct Lane {
    state: State,
    held: rest::Held,
    audio: Vec<f32>,
}

/// What every part of one device block reads.
struct Block<'a> {
    command: &'a Command,
    rate: u32,
    channels: usize,
    frames: usize,
    /// The first block of a new command: all parts restart from the same instant.
    fresh: bool,
    diagnostics: Option<&'a Counters>,
    failed: &'a AtomicBool,
}
impl Block<'_> {
    fn fail(&self, stage: u64) {
        if let Some(d) = self.diagnostics {
            d.failure(stage);
        }
        self.failed.store(true, Ordering::Relaxed);
    }
}

pub(super) struct Mixer {
    pub diagnostics: Option<std::sync::Arc<Counters>>,
    channels: usize,
    rate: u32,
    mixed: Vec<f32>,
    lanes: Vec<Lane>,
    seen: u64,
    /// Frames since the latest command.
    pub elapsed: u64,
    master: Option<cat_smf_editor::host::ProcessorRef>,
    auto_gain: crate::play::auto_gain::MasterAutoGain,
    pool: pool::Pool,
    rest: rest::Rest,
    /// Shared with the UI, which slows its repaints while every stream rests.
    pub resting: std::sync::Arc<rest::Resting>,
}
impl Mixer {
    /// Parts render on up to half the logical CPUs, the calling thread included.
    pub fn new(
        rate: u32,
        channels: usize,
        tracks: usize,
        master: Option<cat_smf_editor::host::ProcessorRef>,
    ) -> Self {
        let cpus = std::thread::available_parallelism().map_or(1, usize::from);
        let participants = tracks.min((cpus / 2).max(1)).max(1);
        Self::with_helpers(rate, channels, tracks, master, participants - 1)
    }

    fn with_helpers(
        rate: u32,
        channels: usize,
        tracks: usize,
        master: Option<cat_smf_editor::host::ProcessorRef>,
        helpers: usize,
    ) -> Self {
        Self {
            diagnostics: None,
            channels,
            rate,
            mixed: vec![0.0; 1024 * channels],
            lanes: (0..tracks)
                .map(|_| Lane {
                    state: State::default(),
                    held: Default::default(),
                    audio: vec![0.0; 1024 * channels],
                })
                .collect(),
            seen: 0,
            elapsed: 0,
            master,
            auto_gain: Default::default(),
            pool: pool::Pool::new(helpers),
            rest: rest::Rest::new(rate),
            resting: Default::default(),
        }
    }

    /// For a real-time device: lets this thread and the part helpers preempt ordinary work.
    pub fn boost(&self) {
        self.pool.boost();
    }

    /// Instrument fragments retain their exact event boundaries. Track and master
    /// effects process the assembled device block, with no additional buffering latency.
    pub fn block(
        &mut self,
        renders: &mut [Render],
        command: &Command,
        max: usize,
        failed: &AtomicBool,
    ) -> &[f32] {
        self.render_block(renders, command, max, failed)
    }

    fn render_block<R: Instrument + Send, E: Effect + Send>(
        &mut self,
        renders: &mut [Render<R, E>],
        command: &Command,
        max: usize,
        failed: &AtomicBool,
    ) -> &[f32] {
        let frames = max.min(1024);
        if frames == 0 {
            return &self.mixed[..0];
        }
        let fresh = self.seen != command.generation;
        self.seen = command.generation;
        if fresh {
            self.elapsed = 0;
        }
        let channels = self.channels;
        let rate = self.rate;
        let count = renders.len().min(self.lanes.len());
        let waiting =
            fresh || rest::waiting(command, &self.lanes[..count], std::time::Instant::now());
        let resting = self.rest.resting(waiting);
        self.resting.store(command.generation, resting);
        if resting {
            // Nothing to play and the tails have ended: no plugin, effect or master runs.
            self.pool.rest();
            if self.rest.skip()
                && let Some(d) = &self.diagnostics
            {
                d.rest();
            }
            self.elapsed = self.elapsed.saturating_add(frames as u64);
            let mixed = &mut self.mixed[..frames * channels];
            mixed.fill(0.0);
            return mixed;
        }
        let block = Block {
            command,
            rate,
            channels,
            frames,
            fresh,
            diagnostics: self.diagnostics.as_deref(),
            failed,
        };
        let parts = Parts(renders.as_mut_ptr());
        let lanes = Parts(self.lanes.as_mut_ptr());
        self.pool.run(count, &|i| {
            // SAFETY: the pool calls each index below `count` exactly once per block and
            // returns only after all have finished, so these borrows are disjoint and
            // end before `renders` and `self.lanes` are touched again.
            let (render, lane) = unsafe { (&mut *parts.at(i), &mut *lanes.at(i)) };
            let rendered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                render_part(i, render, lane, &block)
            }));
            if rendered.is_err() {
                block.fail(4);
                lane.audio[..frames * channels].fill(0.0);
            }
        });
        self.elapsed = self.elapsed.saturating_add(frames as u64);
        let mixed = &mut self.mixed[..frames * channels];
        mixed.fill(0.0);
        for (i, lane) in self.lanes.iter().enumerate().take(count) {
            let audio = &lane.audio[..mixed.len()];
            if let Some(d) = &self.diagnostics {
                d.track(i, audio);
            }
            if !mix_into(mixed, audio, command.gains.get(i).copied().unwrap_or(1.0)) {
                if let Some(d) = &self.diagnostics {
                    d.failure(16);
                }
                failed.store(true, Ordering::Relaxed);
            }
        }
        if let Some(master) = &mut self.master
            && !master.process_input(mixed, channels)
        {
            if let Some(d) = &self.diagnostics {
                d.failure(32);
            }
            failed.store(true, Ordering::Relaxed);
            mixed.fill(0.0);
        }
        if !mixed.iter().all(|s| s.is_finite()) {
            if let Some(d) = &self.diagnostics {
                d.failure(64);
            }
            failed.store(true, Ordering::Relaxed);
            mixed.fill(0.0);
        }
        self.auto_gain
            .process(mixed, rate, channels, !command.auto_gain_disabled);
        self.rest.heard(waiting, mixed, frames);
        if let Some(click) = command.parts.iter().find_map(|p| p.click.as_ref()) {
            click.render(mixed, channels, rate, std::time::Instant::now());
        }
        mixed
    }
}

/// The parts of one block, each handed to exactly one thread at a time.
struct Parts<T>(*mut T);
// SAFETY: `Mixer::render_block` derefs only distinct indices, one thread per index.
unsafe impl<T: Send> Sync for Parts<T> {}
impl<T> Parts<T> {
    /// A method rather than a field read, so closures capture the `Sync` wrapper.
    fn at(&self, index: usize) -> *mut T {
        // SAFETY: callers pass an index below the slice length the pointer came from.
        unsafe { self.0.add(index) }
    }
}

/// One part's whole block: its instruments, split only at its own event and automation
/// boundaries, then its track effects over the full block.
fn render_part<R: Instrument, E: Effect>(
    index: usize,
    render: &mut Render<R, E>,
    lane: &mut Lane,
    block: &Block,
) {
    let channels = block.channels;
    let part = block.command.parts.get(index);
    let state = &mut lane.state;
    let held = &mut lane.held;
    if block.fresh {
        // Stop the previous command before delivering the new event stream.
        state.cursor = Cursor::default();
        *held = Default::default();
        render.reset();
        render.cue(part.and_then(|part| part.take.clone()));
    }
    let mut offset = 0;
    while offset < block.frames {
        let fresh = block.fresh && offset == 0;
        let remaining = block.frames - offset;
        let frames = part.map_or(remaining, |part| {
            boundary(state, part, block.rate, remaining)
        });
        let out = &mut lane.audio[offset * channels..(offset + frames) * channels];
        let frame = state.cursor.frame;
        let changed = if let Some(part) = part {
            if let Some(timeline) = &part.timeline
                && part.take.is_none()
            {
                state.cursor.finished |= frame >= timeline.end;
                state.cursor.frame = frame.saturating_add(frames as u64);
                0
            } else if let Some(sequence) = &part.sequence {
                let origin = part
                    .timeline
                    .as_ref()
                    .map_or(sequence.origin, |timeline| timeline.origin);
                let (_, changed) = state.cursor.block_aligned(
                    sequence,
                    block.rate,
                    frames,
                    &mut state.velocities,
                    &mut state.durations,
                    origin,
                );
                if fresh {
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
        let plan = part.and_then(|p| p.automation.as_deref()).filter(|_| {
            part.and_then(|p| p.timeline.as_ref())
                .map_or(!state.cursor.finished, |t| frame <= t.end)
        });
        let timeline = part.and_then(|part| part.timeline.as_deref());
        let rendered = render.instruments(|render| {
            if let Some(part) = part.filter(|part| part.live.is_some()) {
                // Release the queue lock before calling any plugin code. Batches
                // preserve fast release/retrigger pairs between audio callbacks.
                let mut words = [0u32; 128];
                let now = std::time::Instant::now();
                let phrase = part
                    .phrase
                    .as_ref()
                    .map_or(0, |p| p.due(now, &mut words[..64]));
                let count = phrase
                    + part
                        .live
                        .as_ref()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .take(now, &mut words[phrase..]);
                for &word in &words[..count] {
                    held.note(word);
                }
                render.process_live(&words[..count], out, channels)
            } else if let Some(timeline) = timeline
                && part.is_none_or(|part| part.take.is_none())
            {
                render.process_scheduled(timeline, plan, frame, out, channels)
            } else {
                render.automate(plan, frame)
                    && render.process(&state.velocities, &state.durations, changed, out, channels)
            }
        });
        if !rendered {
            block.fail(4);
            out.fill(0.0);
        }
        offset += frames;
    }
    if !render.apply_effects(&mut lane.audio[..block.frames * channels], channels) {
        block.fail(4);
    }
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

/// Frames until this part's next note event, timeline event or automation grid point.
fn boundary(state: &State, part: &Part, rate: u32, max: usize) -> usize {
    let Some(sequence) = part.sequence.as_ref() else {
        return max;
    };
    if let Some(timeline) = &part.timeline
        && part.take.is_none()
    {
        let count = timeline.next(state.cursor.frame, max);
        return part
            .automation
            .as_ref()
            .map_or(count, |plan| plan.next(state.cursor.frame, count));
    }
    let mut cursor = state.cursor.clone();
    let mut velocities = [0; 128];
    let mut durations = [0; 128];
    let origin = part
        .timeline
        .as_ref()
        .map_or(sequence.origin, |timeline| timeline.origin);
    let notes = cursor
        .block_aligned(sequence, rate, max, &mut velocities, &mut durations, origin)
        .0;
    let notes = part
        .timeline
        .as_ref()
        .map_or(notes, |timeline| timeline.next(state.cursor.frame, notes));
    if !state.cursor.finished {
        part.automation
            .as_ref()
            .map_or(notes, |plan| plan.next(state.cursor.frame, notes))
    } else {
        notes
    }
}

#[cfg(test)]
#[path = "audition_mix/tests.rs"]
mod tests;
