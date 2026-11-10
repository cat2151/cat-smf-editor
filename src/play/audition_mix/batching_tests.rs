use super::*;
use crate::play::{
    audio_sequence::{Note, Sequence},
    automation::{OwnerPlan, RuntimePlan},
    event_timeline::Timeline,
};
use cat_smf_editor::{
    host::parameters::AutomationEvent, instrument_layers::Layer, score::audition_timing::TempoMap,
};
use std::sync::{Arc, Mutex};

struct Gate {
    frame: usize,
    on: bool,
    events: Arc<Mutex<Vec<(usize, bool)>>>,
}
impl Instrument for Gate {
    fn ordered(
        &mut self,
        events: &[AutomationEvent],
        out: &mut [f32],
        channels: usize,
    ) -> Result<(), i32> {
        for event in events.iter().filter(|e| e.kind == 1) {
            self.on = event.midi >> 20 & 15 == 9 && event.midi & 127 != 0;
            self.events.lock().unwrap().push((self.frame, self.on));
        }
        out.fill(if self.on { 0.25 } else { 0.0 });
        self.frame += out.len() / channels;
        Ok(())
    }
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, _: &mut [f32], _: usize) -> bool {
        panic!("scheduled events must retain their explicit MIDI path")
    }
}
struct Delay {
    previous: [f32; 2],
    sizes: Arc<Mutex<Vec<usize>>>,
}
impl Effect for Delay {
    fn apply(&mut self, out: &mut [f32], channels: usize) -> bool {
        assert_eq!(channels, 2);
        self.sizes.lock().unwrap().push(out.len() / channels);
        for frame in out.chunks_exact_mut(channels) {
            let input = [frame[0], frame[1]];
            frame.copy_from_slice(&self.previous);
            self.previous = input;
        }
        true
    }
}

#[test]
fn effects_receive_device_blocks_while_event_fragments_keep_sample_positions() {
    let mut parts = vec![];
    let mut renders = vec![];
    let mut records = vec![];
    for (index, (start, duration)) in [(0.001, 0.095), (0.010, 0.100)].into_iter().enumerate() {
        let sequence = Sequence::new(
            vec![Note {
                start,
                duration,
                pitch: 60,
                velocity: 100,
            }],
            0.512,
        );
        let owner = Arc::new(OwnerPlan {
            owner: index,
            curves: vec![],
            boundaries: vec![],
            midi: false,
            tempo: Arc::new(TempoMap::new(&cat_smf_editor::edit::tracks::new_song(), 0)),
            windows: Default::default(),
            live: Default::default(),
        });
        parts.push(Part {
            timeline: Some(Arc::new(Timeline::new(&sequence, 0.0, 1000))),
            automation: (index == 0)
                .then(|| Arc::new(RuntimePlan::new(owner, &sequence, 0.0, 1000))),
            sequence: Some(Arc::new(sequence)),
            ..Part::default()
        });
        let events = Arc::new(Mutex::new(vec![]));
        let sizes = Arc::new(Mutex::new(vec![]));
        renders.push(Render::new(
            vec![(
                Gate {
                    frame: 0,
                    on: false,
                    events: events.clone(),
                },
                Layer::default(),
            )],
            Some(Delay {
                previous: [0.0; 2],
                sizes: sizes.clone(),
            }),
        ));
        records.push((events, sizes));
    }
    let command = Command {
        generation: 1,
        parts: Arc::new(parts),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let failed = AtomicBool::new(false);
    let mut mixer = Mixer::new(1000, 2, 2, None);
    let mut output = mixer
        .render_block(&mut renders, &command, 80, &failed)
        .to_vec();
    output.extend_from_slice(mixer.render_block(&mut renders, &command, 37, &failed));
    for (frame, sample) in output.as_chunks::<2>().0.iter().enumerate() {
        let expected = 0.25
            * (usize::from((2..97).contains(&frame)) + usize::from((11..111).contains(&frame)))
                as f32;
        assert_eq!(*sample, [expected; 2], "frame {frame}");
    }
    assert_eq!(*records[0].0.lock().unwrap(), [(1, true), (96, false)]);
    assert_eq!(*records[1].0.lock().unwrap(), [(10, true), (110, false)]);
    for (_, sizes) in records {
        assert_eq!(*sizes.lock().unwrap(), [80, 37]);
    }
    assert_eq!(mixer.elapsed, 117);
    assert!(!failed.load(Ordering::Relaxed));
}

#[test]
fn empty_device_request_does_not_process_effects_or_advance_the_clock() {
    let sizes = Arc::new(Mutex::new(vec![]));
    let mut renders = [Render::<Gate, Delay>::new(
        vec![],
        Some(Delay {
            previous: [0.0; 2],
            sizes: sizes.clone(),
        }),
    )];
    let mut mixer = Mixer::new(48000, 2, 1, None);
    assert!(mixer
        .render_block(
            &mut renders,
            &Command::default(),
            0,
            &AtomicBool::new(false)
        )
        .is_empty());
    assert!(sizes.lock().unwrap().is_empty());
    assert_eq!(mixer.elapsed, 0);
}

struct InvalidEffect;
impl Effect for InvalidEffect {
    fn apply(&mut self, out: &mut [f32], _: usize) -> bool {
        out.fill(f32::NAN);
        true
    }
}

#[test]
fn a_nonfinite_post_fragment_effect_is_reported_and_silenced() {
    let mut renders = [Render::<Gate, InvalidEffect>::new(
        vec![],
        Some(InvalidEffect),
    )];
    let mut mixer = Mixer::new(48000, 2, 1, None);
    let failed = AtomicBool::new(false);
    assert_eq!(
        mixer.render_block(&mut renders, &Command::default(), 37, &failed),
        [0.0; 74]
    );
    assert!(failed.load(Ordering::Relaxed));
}
