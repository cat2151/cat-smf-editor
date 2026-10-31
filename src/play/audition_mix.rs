//! One mixing step shared by the output stream and offline rendering.
use super::{Command, Part};
use crate::play::audio_sequence::Cursor;
use crate::play::layer_render::Render;
use std::sync::atomic::{AtomicBool, Ordering};

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

pub(super) struct Mixer {
    pub diagnostics: Option<std::sync::Arc<crate::play::audio_diagnostics::Counters>>,
    channels: usize,
    rate: u32,
    scratch: Vec<f32>,
    mixed: Vec<f32>,
    states: Vec<State>,
    seen: u64,
    /// Frames since the latest command.
    pub elapsed: u64,
    master: Option<cat_smf_editor::host::ProcessorRef>,
    auto_gain: crate::play::auto_gain::MasterAutoGain,
}
impl Mixer {
    pub fn new(
        rate: u32,
        channels: usize,
        tracks: usize,
        master: Option<cat_smf_editor::host::ProcessorRef>,
    ) -> Self {
        Self {
            diagnostics: None,
            channels,
            rate,
            scratch: vec![0.0; 1024 * channels],
            mixed: vec![0.0; 1024 * channels],
            states: (0..tracks).map(|_| State::default()).collect(),
            seen: 0,
            elapsed: 0,
            master,
            auto_gain: Default::default(),
        }
    }

    /// Mixes at most `max` (≤ 1024) frames, stopping early at the next note boundary.
    pub fn block(
        &mut self,
        renders: &mut [Render],
        command: &Command,
        max: usize,
        failed: &AtomicBool,
    ) -> &[f32] {
        let channels = self.channels;
        let rate = self.rate;
        let fresh = self.seen != command.generation;
        self.seen = command.generation;
        if fresh {
            self.elapsed = 0;
            for state in &mut self.states {
                state.cursor = Cursor::default();
            }
        }
        let frames = boundary(&self.states, &command.parts, rate, max.min(1024));
        let len = frames * channels;
        let (scratch, mixed) = (&mut self.scratch[..len], &mut self.mixed[..len]);
        mixed.fill(0.0);
        for (i, (render, state)) in renders.iter_mut().zip(&mut self.states).enumerate() {
            let frame = state.cursor.frame;
            if fresh {
                // Stop the previous command before delivering the new event stream.
                render.reset();
                render.cue(command.parts.get(i).and_then(|part| part.take.clone()));
            }
            let changed = if let Some(part) = command.parts.get(i) {
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
                        rate,
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
            let plan = command
                .parts
                .get(i)
                .and_then(|p| p.automation.as_deref())
                .filter(|_| {
                    command
                        .parts
                        .get(i)
                        .and_then(|p| p.timeline.as_ref())
                        .map_or(!state.cursor.finished, |t| frame <= t.end)
                });
            let timeline = command
                .parts
                .get(i)
                .and_then(|part| part.timeline.as_deref());
            let rendered =
                if let Some(live) = command.parts.get(i).and_then(|part| part.live.as_ref()) {
                    // Release the queue lock before calling any plugin code. Batches
                    // preserve fast release/retrigger pairs between audio callbacks.
                    let mut words = [0u32; 128];
                    let count = {
                        let mut queue = live.lock().unwrap();
                        let count = queue.len().min(words.len());
                        for word in &mut words[..count] {
                            *word = queue.pop_front().unwrap();
                        }
                        count
                    };
                    render.process_live(&words[..count], scratch, channels)
                } else if let Some(timeline) = timeline
                    && command.parts.get(i).is_none_or(|part| part.take.is_none())
                {
                    render.process_scheduled(timeline, plan, frame, scratch, channels)
                } else {
                    render.automate(plan, frame)
                        && render.process(
                            &state.velocities,
                            &state.durations,
                            changed,
                            scratch,
                            channels,
                        )
                };
            if !rendered {
                if let Some(d) = &self.diagnostics {
                    d.failure(4);
                }
                failed.store(true, Ordering::Relaxed);
                scratch.fill(0.0);
            }
            if let Some(d) = &self.diagnostics {
                d.track(i, scratch);
            }
            if !mix_into(mixed, scratch, command.gains.get(i).copied().unwrap_or(1.0)) {
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
        if let Some(click) = command.parts.iter().find_map(|p| p.click.as_ref()) {
            click.render(mixed, channels, rate, std::time::Instant::now());
        }
        self.elapsed = self.elapsed.saturating_add(frames as u64);
        mixed
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

fn boundary(states: &[State], parts: &[Part], rate: u32, max: usize) -> usize {
    states
        .iter()
        .zip(parts)
        .filter_map(|(state, part)| {
            let sequence = part.sequence.as_ref()?;
            if let Some(timeline) = &part.timeline
                && part.take.is_none()
            {
                let count = timeline.next(state.cursor.frame, max);
                return Some(
                    part.automation
                        .as_ref()
                        .map_or(count, |plan| plan.next(state.cursor.frame, count)),
                );
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
            Some(if !state.cursor.finished {
                part.automation
                    .as_ref()
                    .map_or(notes, |plan| plan.next(state.cursor.frame, notes))
            } else {
                notes
            })
        })
        .min()
        .unwrap_or(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::audio_sequence::{Note, Sequence};
    use std::sync::Arc;
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
            sequence: Some(Arc::new(Sequence::new(
                vec![Note {
                    start,
                    duration: start,
                    pitch: 60,
                    velocity: 100,
                }],
                start * 2.0,
            ))),
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
