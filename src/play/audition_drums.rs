//! Replace future drum events without issuing a new playback generation.
use super::super::Audition;
use crate::play::{
    audio_sequence::{MidiEvent, Sequence},
    event_timeline::Timeline,
};
use cat_smf_editor::score::instrument::Patch;
use std::{
    collections::{HashMap, VecDeque},
    sync::{atomic::Ordering, Arc},
};

impl Audition {
    pub(crate) fn reload_drums(
        &mut self,
        track: usize,
        patch: &Patch,
        sequence: &Sequence,
        origin: f64,
    ) {
        let Some(voice) = &mut self.voice else { return };
        let Some(index) = voice.patches.iter().position(|p| p.0 == track) else {
            return;
        };
        // The callback takes these locks in this order and reads the command only
        // after acquiring the render boundary. Routing and events change together.
        let mut renders = voice.renders.lock().unwrap();
        let mut command = voice.mailbox.lock().unwrap();
        if !replace_part(&mut command, index, sequence, origin, voice.rate) {
            return;
        }
        renders[index].set_drums(patch.drums.clone());
        for layer in &patch.layers {
            renders[index].set_layer_level(layer.settings.id, layer.settings.volume_db);
            renders[index].set_layer_mute(layer.settings.id, layer.settings.mute);
        }
        voice.natives[index].set_drum_controls(patch);
        voice.patches[index].1 = patch.clone();
    }
}

fn replace_part(
    command: &mut super::super::Command,
    index: usize,
    sequence: &Sequence,
    origin: f64,
    rate: u32,
) -> bool {
    let Some(old) = command.parts.get(index).and_then(|p| p.sequence.as_ref()) else {
        return false;
    };
    let elapsed = command.progress.load(Ordering::Relaxed) as f64 / f64::from(rate);
    let sequence = splice(old, sequence, elapsed, rate);
    let mut parts = command.parts.as_ref().clone();
    let part = &mut parts[index];
    part.timeline = Some(Arc::new(Timeline::new(&sequence, origin, rate)));
    if let Some(plan) = &part.automation {
        part.automation = Some(Arc::new(crate::play::automation::RuntimePlan::new(
            plan.owner.clone(),
            &sequence,
            origin,
            rate,
        )));
    }
    part.sequence = Some(Arc::new(sequence));
    command.parts = Arc::new(parts);
    true
}

fn selected(
    sequence: &Sequence,
    elapsed: f64,
    rate: u32,
    past: bool,
    history: Option<&Sequence>,
) -> Vec<MidiEvent> {
    let cutoff = crate::play::event_timeline::frame(elapsed, rate);
    let mut starts: HashMap<u32, VecDeque<bool>> = HashMap::new();
    let mut previous: HashMap<u32, VecDeque<bool>> = HashMap::new();
    if let Some(history) = history {
        for event in &history.events {
            if (event.word >> 20) & 15 == 9 && event.word & 127 != 0 {
                let before = crate::play::event_timeline::frame(event.time, rate)
                    .saturating_sub(crate::play::event_timeline::frame(history.origin, rate))
                    < cutoff;
                previous
                    .entry(event.word & 0x000f_7f00)
                    .or_default()
                    .push_back(before);
            }
        }
    }
    sequence
        .events
        .iter()
        .filter(|event| {
            let before = crate::play::event_timeline::frame(event.time, rate)
                .saturating_sub(crate::play::event_timeline::frame(sequence.origin, rate))
                < cutoff;
            let status = (event.word >> 20) & 15;
            let key = event.word & 0x000f_7f00;
            let on = status == 9 && event.word & 127 != 0;
            if on {
                let before = previous
                    .get_mut(&key)
                    .and_then(|queue| queue.pop_front())
                    .unwrap_or(before);
                starts.entry(key).or_default().push_back(before);
                before == past
            } else if status == 8 || status == 9 {
                starts
                    .get_mut(&key)
                    .and_then(|queue| queue.pop_front())
                    .unwrap_or(before)
                    == past
            } else {
                before == past
            }
        })
        .cloned()
        .collect()
}

fn splice(old: &Sequence, new: &Sequence, elapsed: f64, rate: u32) -> Sequence {
    let mut result = new.clone();
    // Keep releases belonging to already-started hits, even when swing moves
    // their newly computed release behind the current audio frame.
    result.events = selected(old, elapsed, rate, true, None);
    result
        .events
        .extend(selected(new, elapsed, rate, false, Some(old)));
    result.events.sort_by(|a, b| a.time.total_cmp(&b.time));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::audio_sequence::Note;

    #[test]
    fn replacement_keeps_mixer_clock_generation_other_tracks_and_progress_identity() {
        use super::super::super::{mix::Mixer, Command, Part};
        use crate::play::layer_render::Render;
        use std::sync::atomic::AtomicBool;
        let rate = 48000;
        let old = Sequence::new(
            vec![Note {
                start: 0.1,
                duration: 0.02,
                pitch: 36,
                velocity: 100,
            }],
            1.0,
        );
        let mut new = old.clone();
        new.events[0].word = (new.events[0].word & !127) | 81;
        let part = Part {
            timeline: Some(Arc::new(Timeline::new(&old, 0.0, rate))),
            sequence: Some(Arc::new(old)),
            ..Part::default()
        };
        let mut command = Command {
            generation: 7,
            parts: Arc::new(vec![part.clone(), part]),
            ..Command::default()
        };
        let progress = command.progress.clone();
        let other = command.parts[1].timeline.as_ref().unwrap().clone();
        let mut mixer = Mixer::new(rate, 2, 2, None);
        let mut renders = vec![Render::new(vec![], None), Render::new(vec![], None)];
        let failed = AtomicBool::new(false);
        mixer.block(&mut renders, &command, 1024, &failed);
        command.progress.store(mixer.elapsed, Ordering::Relaxed);
        assert!(replace_part(&mut command, 0, &new, 0.0, rate));
        assert_eq!(command.generation, 7);
        assert!(Arc::ptr_eq(&progress, &command.progress));
        assert!(Arc::ptr_eq(
            &other,
            command.parts[1].timeline.as_ref().unwrap()
        ));
        assert_eq!(
            command.parts[0].timeline.as_ref().unwrap().events[0].word & 127,
            81
        );
        mixer.block(&mut renders, &command, 1024, &failed);
        assert_eq!(mixer.elapsed, 2048);
        assert!(!failed.load(Ordering::Relaxed));
    }

    #[test]
    fn moving_a_started_hit_ahead_of_now_does_not_retrigger_it() {
        let old = Sequence::new(
            vec![Note {
                start: 0.1,
                duration: 0.02,
                pitch: 36,
                velocity: 100,
            }],
            1.0,
        );
        let new = Sequence::new(
            vec![Note {
                start: 0.2,
                duration: 0.02,
                pitch: 36,
                velocity: 81,
            }],
            1.0,
        );
        let result = splice(&old, &new, 0.15, 48000);
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.events[0].time, 0.1);
    }

    #[test]
    fn swing_replacement_keeps_started_hits_and_their_original_releases() {
        let old = Sequence::new(
            vec![
                Note {
                    start: 0.1,
                    duration: 0.2,
                    pitch: 36,
                    velocity: 100,
                },
                Note {
                    start: 0.5,
                    duration: 0.1,
                    pitch: 36,
                    velocity: 100,
                },
            ],
            1.0,
        );
        let new = Sequence::new(
            vec![
                Note {
                    start: 0.05,
                    duration: 0.1,
                    pitch: 36,
                    velocity: 80,
                },
                Note {
                    start: 0.6,
                    duration: 0.1,
                    pitch: 36,
                    velocity: 80,
                },
            ],
            1.0,
        );
        let result = splice(&old, &new, 0.2, 48000);
        let events: Vec<_> = result
            .events
            .iter()
            .map(|e| (e.time, e.word & 127))
            .collect();
        assert_eq!(events.len(), 4);
        assert_eq!(events[0], (0.1, 100));
        assert!((events[1].0 - 0.3).abs() < 1e-9);
        assert_eq!(events[2], (0.6, 80));
        // A second edit must retain the original release as well.
        assert_eq!(splice(&result, &new, 0.25, 48000).events.len(), 4);
    }
}
