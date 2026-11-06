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
fn live_mute_silences_only_the_target_layer_and_unmute_restores_gain() {
    let record = Rc::new(RefCell::new(Vec::new()));
    let mut render = Render::<_, NoEffect>::new(
        (1..=2)
            .map(|id| {
                (
                    Recorder(record.clone()),
                    Layer {
                        id,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        None,
    );
    let mut out = [0.0; 2];
    render.set_layer_level(1, -6.0206);
    render.set_layer_mute(1, true);
    assert!(render.process_live(&[0x2090_3c64], &mut out, 2));
    assert_eq!(out, [1.0; 2]);
    render.set_layer_mute(2, true);
    assert!(render.process_live(&[0x2080_3c00], &mut out, 2));
    assert_eq!(out, [0.0; 2]);
    render.set_layer_mute(1, false);
    assert!(render.process_live(&[], &mut out, 2));
    assert!((out[0] - 0.5).abs() < 0.00001);
}

#[test]
fn level_changes_only_audio_gain_while_held_notes_keep_their_velocity_and_release() {
    let record = Rc::new(RefCell::new(Vec::new()));
    let mut render =
        Render::<_, NoEffect>::new(vec![(Recorder(record.clone()), Layer::default())], None);
    render.set_drums(Some(Default::default()));
    let mut out = [0.0; 2];
    assert!(render.process_live(&[0x2099_2464], &mut out, 2));
    assert_eq!(out, [1.0; 2]);
    render.set_layer_level(1, -6.0206);
    assert!(render.process_live(&[], &mut out, 2));
    assert!((out[0] - 0.5).abs() < 0.00001);
    assert!(render.process_live(&[0x2089_2400, 0x2099_2464], &mut out, 2));
    assert!((out[0] - 0.5).abs() < 0.00001);
    assert_eq!(
        *record.borrow(),
        vec![vec![0x2099_3c64], vec![], vec![0x2089_3c00, 0x2099_3c64]]
    );
}

#[test]
fn changing_drum_output_releases_held_pitch_and_uses_new_pitch_for_next_hit() {
    let record = Rc::new(RefCell::new(Vec::new()));
    let mut render =
        Render::<_, NoEffect>::new(vec![(Recorder(record.clone()), Layer::default())], None);
    let mut drums = cat_smf_editor::score::drums::Settings::default();
    render.set_drums(Some(drums.clone()));
    assert!(render.process_live(&[0x2090_2450], &mut [0.; 2], 2));
    drums.rows[0].output = 73;
    render.set_drums(Some(drums));
    assert!(render.process_live(&[0x2080_2400, 0x2090_2451, 0x2080_2400], &mut [0.; 2], 2));
    assert_eq!(
        *record.borrow(),
        vec![
            vec![0x2090_3c50],
            vec![0x2080_3c00, 0x2090_4951, 0x2080_4900]
        ]
    );
}

#[test]
fn live_chords_release_and_fast_retrigger_preserve_order_without_restarting_held_notes() {
    let record = Rc::new(RefCell::new(Vec::new()));
    let mut render =
        Render::<_, NoEffect>::new(vec![(Recorder(record.clone()), Layer::default())], None);
    for words in [
        &[0x2090_3c50][..],
        &[0x2090_4050][..],
        &[0x2080_3c00, 0x2090_3c50][..],
        &[0x2080_4000, 0x2080_3c00][..],
    ] {
        assert!(render.process_live(words, &mut [0.; 64], 2));
    }
    assert_eq!(
        *record.borrow(),
        vec![
            vec![0x2090_3c50],
            vec![0x2090_4050],
            vec![0x2080_3c00, 0x2090_3c50],
            vec![0x2080_4000, 0x2080_3c00]
        ]
    );
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

#[test]
fn kit_actual_notes_share_one_layer_keep_endpoints_and_retrigger_order() {
    let records: Vec<_> = (0..2).map(|_| Rc::new(RefCell::new(Vec::new()))).collect();
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
    render.set_kit(Some(&cat_smf_editor::drum_kit::Kit {
        layer: 2,
        rows: None,
        muted: vec![],
        swing: Default::default(),
        notes: vec![],
    }));
    for words in [
        &[0x2099_0064, 0x2099_7f50][..],
        &[0x2089_7f00, 0x2099_7f70][..],
        &[0x2089_0000, 0x2089_7f00][..],
    ] {
        assert!(render.process_live(words, &mut [0.; 64], 2));
    }
    assert!(records[0].borrow().iter().all(|words| words.is_empty()));
    assert_eq!(
        *records[1].borrow(),
        vec![
            vec![0x2099_0064, 0x2099_7f50],
            vec![0x2089_7f00, 0x2099_7f70],
            vec![0x2089_0000, 0x2089_7f00]
        ]
    );
}

#[test]
fn kit_mute_blocks_only_its_note_on_and_keeps_releases_and_other_channels() {
    let record = Rc::new(RefCell::new(Vec::new()));
    let mut render =
        Render::<_, NoEffect>::new(vec![(Recorder(record.clone()), Layer::default())], None);
    let mut kit = cat_smf_editor::drum_kit::Kit {
        layer: 1,
        notes: vec![],
        rows: None,
        muted: vec![],
        swing: Default::default(),
    };
    render.set_kit(Some(&kit));
    assert!(render.process_live(&[0x2099_2464], &mut [0.; 2], 2));
    kit.muted.push(36);
    render.set_kit(Some(&kit));
    assert!(render.process_live(
        &[0x2089_2400, 0x2099_2464, 0x2099_2664, 0x20b9_4000],
        &mut [0.; 2],
        2
    ));
    assert_eq!(record.borrow()[1], [0x2089_2400, 0x2099_2664, 0x20b9_4000]);
    kit.muted.clear();
    render.set_kit(Some(&kit));
    assert!(render.process_live(&[0x2099_2464], &mut [0.; 2], 2));
    assert_eq!(record.borrow()[2], [0x2099_2464]);
}
