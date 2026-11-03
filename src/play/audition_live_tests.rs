use super::*;
use crate::play::{audition::mix::Mixer, audition_native::Native};
use cat_smf_editor::score::pattern::recording::Modulation;
use std::sync::atomic::AtomicBool;

#[test]
fn keys_carry_their_velocity_and_cc1_follows_the_choice_on_the_session_clock() {
    use std::time::{Duration, Instant};
    let queue = Queue::default();
    push(&queue, &[64, 67], 93);
    push(&queue, &[64, 67], 0);
    let mut keys = queue.lock().unwrap();
    assert_eq!(
        keys.words.iter().copied().collect::<Vec<_>>(),
        [0x2090_405d, 0x2090_435d, 0x2080_4000, 0x2080_4300]
    );
    keys.words.clear();
    let clock = Instant::now();
    let at = |ms| clock + Duration::from_millis(ms);
    let mut out = [0u32; 4];
    // Off sends nothing.
    assert_eq!(keys.take(at(0), &mut out), 0);
    keys.modulate(clock, Modulation::Sweep);
    assert_eq!(keys.take(at(1000), &mut out), 1);
    assert_eq!(out[0], 0x20B0_0100 | 63);
    assert_eq!(keys.take(at(1005), &mut out), 0, "same value is not resent");
    keys.words.push_back(word(60, 80));
    assert_eq!(keys.take(at(2000), &mut out), 2);
    assert_eq!(out[..2], [0x20B0_0100 | 127, word(60, 80)]);
    // Another choice is sent at once, even at the same value.
    keys.modulate(clock, Modulation::Fixed(127));
    assert_eq!(keys.take(at(2000), &mut out), 1);
    assert_eq!(keys.take(at(3000), &mut out), 0);
}

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
            0 => push(&queue, &[60], 80),
            1 => push(&queue, &[64, 67], 80),
            2 => {
                push(&queue, &[60], 0);
                push(&queue, &[60], 80);
            }
            _ => push(&queue, &[60, 64, 67], 0),
        }
        let mut peak = 0f32;
        for _ in 0..100 {
            let out = mix.block(&mut renders, &command, 512, &failed);
            assert!(out.iter().all(|v| v.is_finite()));
            peak = out.iter().fold(peak, |p, v| p.max(v.abs()));
            native.pump();
        }
        assert!(!failed.load(Ordering::Relaxed), "stage {stage}");
        assert!(queue.lock().unwrap().words.is_empty());
        peaks.push(peak);
    }
    assert!(peaks[..3].iter().all(|p| *p > 0.0001), "{peaks:?}");
    let tail = mix.block(&mut renders, &command, 512, &failed);
    assert!(tail.iter().all(|v| v.abs() < 0.0001), "notes remained held");
}

#[test]
#[ignore = "requires installed Surge XT; renders a repeating phrase offline in real time"]
fn native_phrase_sounds_once_per_lap_beside_the_live_queue() {
    use std::time::{Duration, Instant};
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let native = Native::open(48000, &Patch::default()).unwrap();
    let mut renders = [unsafe { native.render() }];
    let phrase = Arc::new(super::super::phrase::Phrase::default());
    let command = Command {
        generation: 1,
        parts: Arc::new(vec![Part {
            live: Some(Queue::default()),
            phrase: Some(phrase.clone()),
            ..Part::default()
        }]),
        gains: Arc::new(vec![1.0]),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let mut mix = Mixer::new(48000, 2, 1, None);
    let failed = AtomicBool::new(false);
    let start = Instant::now();
    let lap = Duration::from_millis(400);
    phrase.play(
        start,
        lap,
        vec![
            (Duration::ZERO, 60, 80),
            (Duration::from_millis(100), 60, 0),
        ],
    );
    // Loud 10 ms blocks, as (time since start, loud).
    let mut loud = vec![];
    while start.elapsed() < lap * 3 + Duration::from_millis(350) {
        let out = mix.block(&mut renders, &command, 480, &failed);
        let peak = out.iter().fold(0f32, |p, v| p.max(v.abs()));
        loud.push((start.elapsed(), peak > 0.001));
        native.pump();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!failed.load(Ordering::Relaxed));
    let onsets: Vec<Duration> = loud
        .windows(2)
        .filter(|w| !w[0].1 && w[1].1)
        .map(|w| w[1].0)
        .chain(loud.first().filter(|f| f.1).map(|f| f.0))
        .collect();
    eprintln!("onsets {onsets:?}");
    assert_eq!(onsets.len(), 4, "{onsets:?}");
}
