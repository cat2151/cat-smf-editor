//! Exercise the production sender with a recording processor, without a DLL/device.
mod host {
    pub use cat_smf_editor::host::parameters;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone, Default)]
    pub struct ProcessorRef(pub Rc<RefCell<Vec<Vec<u32>>>>);
    impl ProcessorRef {
        pub fn process(&mut self, words: &[u32], _: &mut [f32], _: usize) -> bool {
            self.0.borrow_mut().push(words.to_vec());
            true
        }
        pub fn process_automation(
            &mut self,
            events: &[parameters::AutomationEvent],
            out: &mut [f32],
            channels: usize,
        ) -> Result<(), i32> {
            let words: Vec<_> = events
                .iter()
                .filter(|e| e.kind == 1)
                .map(|e| e.midi)
                .collect();
            self.process(&words, out, channels).then_some(()).ok_or(-1)
        }
        pub fn parameters(&mut self, _: &[parameters::ParameterValue]) -> Result<(), i32> {
            Ok(())
        }
    }
}
#[allow(dead_code)]
#[path = "../../cat-plugin-player/shared/note_render.rs"]
mod sender;
use host::parameters::{AutomationEvent, ParameterValue};

#[test]
fn explicit_events_reach_the_processor_in_order_without_countdown_offs() {
    let processor = host::ProcessorRef::default();
    let log = processor.0.clone();
    let mut sender = sender::Render::new(processor);
    let mut out = [0.0; 2048];
    sender
        .process_events(&[AutomationEvent::note(0x2092_4864)], &mut out, 2)
        .unwrap();
    sender.process_events(&[], &mut out, 2).unwrap();
    sender
        .process_events(
            &[
                AutomationEvent::note(0x2082_482d),
                AutomationEvent::note(0x2092_3c5a),
            ],
            &mut out,
            2,
        )
        .unwrap();
    assert_eq!(
        *log.borrow(),
        vec![vec![0x2092_4864], vec![], vec![0x2082_482d, 0x2092_3c5a]]
    );
}

#[test]
fn replacement_and_stop_release_all_channels_and_repeated_same_pitch_notes() {
    let processor = host::ProcessorRef::default();
    let log = processor.0.clone();
    let mut sender = sender::Render::new(processor);
    let mut out = [0.0; 2];
    sender
        .process_events(
            &[
                AutomationEvent::note(0x2092_4864),
                AutomationEvent::note(0x2092_4864),
                AutomationEvent::note(0x2093_4864),
            ],
            &mut out,
            2,
        )
        .unwrap();
    sender.reset_events();
    sender
        .process_events(
            &[
                AutomationEvent::parameter(ParameterValue::default()),
                AutomationEvent::note(0x2090_3c64),
            ],
            &mut out,
            2,
        )
        .unwrap();
    assert_eq!(
        log.borrow()[1],
        vec![0x2082_4800, 0x2082_4800, 0x2083_4800, 0x2090_3c64]
    );
    assert!(sender.process(&[0; 128], &[0; 128], 3, &mut out, 2));
    assert_eq!(log.borrow()[2], vec![0x2080_3c00]);
}

#[test]
fn live_audition_replacement_is_pitch_independent_and_zero_duration_is_not_stuck() {
    let processor = host::ProcessorRef::default();
    let log = processor.0.clone();
    let mut sender = sender::Render::new(processor);
    let mut out = [0.0; 20];
    let mut v = [0; 128];
    v[72] = 100;
    assert!(sender.process(&v, &[u64::MAX; 128], 1, &mut out, 2));
    v[72] = 0;
    v[60] = 90;
    assert!(sender.process(&v, &[u64::MAX; 128], 1, &mut out, 2));
    assert_eq!(log.borrow()[1], vec![0x2080_4800, 0x2090_3c5a]);
    assert!(sender.process(&v, &[0; 128], 1, &mut out, 2));
    assert_eq!(log.borrow()[2], vec![0x2080_3c00, 0x2090_3c5a, 0x2080_3c00]);
    assert!(sender.process(&[0; 128], &[0; 128], 3, &mut out, 2));
    assert!(log.borrow()[3].is_empty());
}
