use super::*;
use crate::play::{audition::mix::Mixer, audition_native::Native};
use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "requires installed Surge XT; renders the live MIDI queue offline without audio output"]
fn native_live_queue_sounds_chords_and_releases_to_silence() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let native = Native::open(48000, &Patch::default()).unwrap();
    let mut renders = [unsafe { native.render() }];
    let queue = Queue::default();
    let command = Command {
        generation: 1,
        parts: Arc::new(vec![Part {
            live: Some(queue.clone()),
            ..Part::default()
        }]),
        gains: Arc::new(vec![1.0]),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let mut mix = Mixer::new(48000, 2, 1, None);
    let failed = AtomicBool::new(false);
    let mut peaks = vec![];
    for stage in 0..4 {
        match stage {
            0 => push(&queue, 60, true),
            1 => {
                push(&queue, 64, true);
                push(&queue, 67, true);
            }
            2 => {
                push(&queue, 60, false);
                push(&queue, 60, true);
            }
            _ => {
                for pitch in [60, 64, 67] {
                    push(&queue, pitch, false);
                }
            }
        }
        let mut peak = 0f32;
        for _ in 0..100 {
            let out = mix.block(&mut renders, &command, 512, &failed);
            assert!(out.iter().all(|v| v.is_finite()));
            peak = out.iter().fold(peak, |p, v| p.max(v.abs()));
            native.pump();
        }
        assert!(!failed.load(Ordering::Relaxed), "stage {stage}");
        assert!(queue.lock().unwrap().is_empty());
        peaks.push(peak);
    }
    assert!(peaks[..3].iter().all(|p| *p > 0.0001), "{peaks:?}");
    let tail = mix.block(&mut renders, &command, 512, &failed);
    assert!(tail.iter().all(|v| v.abs() < 0.0001), "notes remained held");
}
