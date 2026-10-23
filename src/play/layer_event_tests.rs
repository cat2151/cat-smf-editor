use super::*;
use crate::play::{
    audio_sequence::{Note, Sequence},
    event_timeline::Timeline,
};
use cat_smf_editor::host::parameters::AutomationEvent;
use std::{cell::RefCell, rc::Rc};

struct Recorder(Rc<RefCell<Vec<Vec<u32>>>>);
impl Instrument for Recorder {
    fn ordered(
        &mut self,
        events: &[AutomationEvent],
        out: &mut [f32],
        _: usize,
    ) -> Result<(), i32> {
        self.0.borrow_mut().push(
            events
                .iter()
                .filter(|e| e.kind == 1)
                .map(|e| e.midi)
                .collect(),
        );
        out.fill(1.0);
        Ok(())
    }
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, _: &mut [f32], _: usize) -> bool {
        panic!("scheduled notes must never use the duration renderer")
    }
}
struct NoEffect;
impl Effect for NoEffect {
    fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
        true
    }
}

#[test]
fn every_layer_receives_the_complete_event_order_after_transposition() {
    let first = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(Vec::new()));
    let mut render = Render::<_, NoEffect>::new(
        vec![
            (Recorder(first.clone()), Layer::default()),
            (
                Recorder(second.clone()),
                Layer {
                    octave: 1,
                    ..Layer::default()
                },
            ),
        ],
        None,
    );
    let seq = Sequence::new(
        vec![
            Note {
                start: 0.0,
                duration: 0.00286,
                pitch: 72,
                velocity: 100,
            },
            Note {
                start: 0.00286,
                duration: 0.00286,
                pitch: 60,
                velocity: 90,
            },
        ],
        0.01,
    );
    let timeline = Timeline::new(&seq, 0.0, 48000);
    let mut out = [0.0; 2];
    for sample in [0, 137, 275] {
        assert!(render.process_scheduled(&timeline, None, sample, &mut out, 2));
    }
    assert_eq!(first.borrow()[1], vec![0x2080_4800, 0x2090_3c5a]);
    assert_eq!(second.borrow()[1], vec![0x2080_5400, 0x2090_485a]);
    assert_eq!(first.borrow()[2], vec![0x2080_3c00]);
    assert_eq!(second.borrow()[2], vec![0x2080_4800]);
    assert_eq!(out, [2.0, 2.0]);
}
