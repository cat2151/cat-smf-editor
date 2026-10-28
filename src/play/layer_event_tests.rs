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
fn channel_controls_broadcast_to_drum_layers_without_pitch_routing() {
    let records: Vec<_> = (0..4).map(|_| Rc::new(RefCell::new(Vec::new()))).collect();
    let mut render = Render::<_, NoEffect>::new(
        records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    Recorder(r.clone()),
                    Layer {
                        id: i as u64 + 1,
                        octave: 2,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        None,
    );
    render.set_drums(Some(Default::default()));
    let mut seq = Sequence::new(vec![], 1.0);
    let expected = vec![0x20b3_077f, 0x20e3_0060, 0x20b3_407f];
    seq.events = expected
        .iter()
        .map(|&word| crate::play::audio_sequence::MidiEvent { time: 0., word })
        .collect();
    let timeline = Timeline::new(&seq, 0., 48000);
    assert!(render.process_scheduled(&timeline, None, 0, &mut [0.; 2], 2));
    for record in records {
        assert_eq!(record.borrow()[0], expected);
    }
}

#[test]
fn drum_rows_receive_only_their_note_on_and_off_at_the_configured_pitch() {
    let records: Vec<_> = (0..4).map(|_| Rc::new(RefCell::new(Vec::new()))).collect();
    let mut render = Render::<_, NoEffect>::new(
        records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    Recorder(r.clone()),
                    Layer {
                        id: i as u64 + 1,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        None,
    );
    let mut drums = cat_smf_editor::score::drums::Settings::default();
    for (i, row) in drums.rows.iter_mut().enumerate() {
        row.output = 60 + i as u8;
    }
    render.set_drums(Some(drums));
    let mut pitches = cat_smf_editor::score::drums::NOTES.to_vec();
    pitches.push(72);
    let sequence = Sequence::new(
        pitches
            .iter()
            .map(|&pitch| Note {
                start: 0.0,
                duration: 0.01,
                pitch,
                velocity: 100,
            })
            .collect(),
        0.02,
    );
    let timeline = Timeline::new(&sequence, 0.0, 48000);
    let mut out = [0.0; 2];
    assert!(render.process_scheduled(&timeline, None, 0, &mut out, 2));
    assert!(render.process_scheduled(&timeline, None, 480, &mut out, 2));
    for (i, record) in records.iter().enumerate() {
        let pitch = (60 + i as u32) << 8;
        assert_eq!(
            *record.borrow(),
            vec![vec![0x2090_0064 | pitch], vec![0x2080_0000 | pitch]]
        );
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
