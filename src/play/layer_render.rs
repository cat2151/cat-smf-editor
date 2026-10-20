//! One scheduler fans out; audio merges before the shared track effect.
#[cfg(test)]
#[path = "layer_audibility_tests.rs"]
mod audibility_tests;
use crate::play::sung::{Player, Take};
use cat_smf_editor::{host::ProcessorRef, instrument_layers::Layer, note_render};
use std::sync::Arc;

pub(crate) trait Instrument {
    fn render(
        &mut self,
        velocities: &[u8; 128],
        durations: &[u64; 128],
        changed: i32,
        out: &mut [f32],
        channels: usize,
    ) -> bool;
}
impl Instrument for note_render::Render {
    fn render(
        &mut self,
        v: &[u8; 128],
        d: &[u64; 128],
        c: i32,
        out: &mut [f32],
        channels: usize,
    ) -> bool {
        self.process(v, d, c, out, channels)
    }
}
pub(crate) trait Effect {
    fn apply(&mut self, out: &mut [f32], channels: usize) -> bool;
}
impl Effect for ProcessorRef {
    fn apply(&mut self, out: &mut [f32], channels: usize) -> bool {
        self.process_input(out, channels)
    }
}

pub struct Render<R = note_render::Render, E = ProcessorRef> {
    layers: Vec<(R, Layer)>,
    effects: Option<E>,
    scratch: Vec<f32>,
    /// While it has a take, it silences the instruments and sounds in the first layer's place.
    sung: Player,
    sung_scratch: Vec<f32>,
}

impl<R: Instrument, E: Effect> Render<R, E> {
    pub fn new(layers: Vec<(R, Layer)>, effects: Option<E>) -> Self {
        Self {
            layers,
            effects,
            scratch: vec![0.0; 1024 * 64],
            sung: Player::default(),
            sung_scratch: vec![0.0; 1024 * 64],
        }
    }

    /// At the start of each playback; `None` gives the notes back to the instruments.
    pub fn cue(&mut self, take: Option<Arc<Take>>) {
        self.sung.cue(take);
    }

    pub fn process(
        &mut self,
        velocities: &[u8; 128],
        durations: &[u64; 128],
        changed: i32,
        out: &mut [f32],
        channels: usize,
    ) -> bool {
        if out.len() > self.scratch.len() || channels == 0 || out.len() / channels > 1024 {
            return false;
        }
        out.fill(0.0);
        let sung = self.sung.active();
        let sung_out = &mut self.sung_scratch[..out.len()];
        self.sung.render(velocities, changed, sung_out, channels);
        let any_solo = self.layers.iter().any(|(_, layer)| layer.solo);
        for (index, (render, layer)) in self.layers.iter_mut().enumerate() {
            let (shifted, lengths) = if sung {
                ([0; 128], [0; 128])
            } else {
                transpose(layer, velocities, durations)
            };
            let scratch = &mut self.scratch[..out.len()];
            if !render.render(&shifted, &lengths, changed, scratch, channels) {
                return false;
            }
            let Ok(gain) = layer.gain() else {
                return false;
            };
            if layer.mute || (any_solo && !layer.solo) {
                continue;
            }
            let source: &[f32] = match (sung, index) {
                (false, _) => scratch,
                (true, 0) => sung_out,
                (true, _) => continue,
            };
            for (sum, sample) in out.iter_mut().zip(source) {
                *sum += *sample * gain;
            }
        }
        if sung && self.layers.is_empty() {
            out.copy_from_slice(sung_out);
        }
        if !out.iter().all(|v| v.is_finite())
            || !self
                .effects
                .as_mut()
                .is_none_or(|effects| effects.apply(out, channels))
            || !out.iter().all(|v| v.is_finite())
        {
            out.fill(0.0);
            return false;
        }
        true
    }
}

fn transpose(
    layer: &Layer,
    velocities: &[u8; 128],
    durations: &[u64; 128],
) -> ([u8; 128], [u64; 128]) {
    let mut shifted = [0; 128];
    let mut lengths = [0; 128];
    for pitch in 0..128 {
        if let Some(target) = layer.pitch(pitch as u8) {
            shifted[target as usize] = velocities[pitch];
            lengths[target as usize] = durations[pitch];
        }
    }
    (shifted, lengths)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Mock {
        pitch: usize,
        seen: std::rc::Rc<std::cell::RefCell<Vec<(u8, u64, i32)>>>,
    }
    impl Instrument for Mock {
        fn render(
            &mut self,
            v: &[u8; 128],
            d: &[u64; 128],
            c: i32,
            out: &mut [f32],
            _: usize,
        ) -> bool {
            self.seen
                .borrow_mut()
                .push((v[self.pitch], d[self.pitch], c));
            out.fill(if v[self.pitch] > 0 { 1.0 } else { 0.0 });
            true
        }
    }
    struct MockEffect(std::rc::Rc<std::cell::Cell<usize>>);
    impl Effect for MockEffect {
        fn apply(&mut self, out: &mut [f32], _: usize) -> bool {
            self.0.set(self.0.get() + 1);
            for sample in out {
                *sample += 1.0;
            }
            true
        }
    }
    #[test]
    fn layer_gain_precedes_sum_and_common_effect_runs_once_with_independent_note_inputs() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        let a = Rc::new(RefCell::new(vec![]));
        let b = Rc::new(RefCell::new(vec![]));
        let calls = Rc::new(Cell::new(0));
        let mut render = Render::new(
            vec![
                (
                    Mock {
                        pitch: 60,
                        seen: a.clone(),
                    },
                    Layer {
                        volume_db: -6.0206,
                        ..Default::default()
                    },
                ),
                (
                    Mock {
                        pitch: 72,
                        seen: b.clone(),
                    },
                    Layer {
                        id: 2,
                        octave: 1,
                        ..Default::default()
                    },
                ),
            ],
            Some(MockEffect(calls.clone())),
        );
        let mut v = [0; 128];
        let mut d = [0; 128];
        v[60] = 80;
        d[60] = 2048;
        let mut out = [0.0; 4];
        assert!(render.process(&v, &d, 1, &mut out, 2));
        assert!((out[0] - 2.5).abs() < 0.00001);
        assert_eq!(calls.get(), 1);
        assert_eq!(*a.borrow(), [(80, 2048, 1)]);
        assert_eq!(*b.borrow(), [(80, 2048, 1)]);
        assert!(render.process(&[0; 128], &[0; 128], 3, &mut out, 2));
        assert_eq!(a.borrow()[1], (0, 0, 3));
        assert_eq!(b.borrow()[1], (0, 0, 3));
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn transpose_preserves_velocity_duration_and_drops_out_of_range() {
        let layer = Layer {
            octave: 1,
            ..Default::default()
        };
        let mut velocities = [0; 128];
        let mut durations = [0; 128];
        velocities[60] = 91;
        durations[60] = 1000;
        velocities[120] = 127;
        durations[120] = 500;
        let (v, d) = transpose(&layer, &velocities, &durations);
        assert_eq!((v[72], d[72]), (91, 1000));
        assert_eq!(v.iter().filter(|v| **v != 0).count(), 1);
        assert_eq!(velocities[60], 91);
    }
}
