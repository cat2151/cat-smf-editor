use super::*;
use crate::play::audio_sequence::{Note, Sequence};
use cat_smf_editor::{host::parameters::AutomationEvent, instrument_layers::Layer};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Holds `gain` while a pitch is down, then halves each call, like a release tail.
struct Bell {
    gain: f32,
    level: f32,
    held: usize,
    calls: Arc<AtomicUsize>,
}
impl Bell {
    fn tone(&mut self, held: bool, out: &mut [f32]) {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.level = if held { self.gain } else { self.level * 0.5 };
        out.fill(self.level);
    }
}
impl Instrument for Bell {
    fn ordered(
        &mut self,
        events: &[AutomationEvent],
        out: &mut [f32],
        _: usize,
    ) -> Result<(), i32> {
        for event in events.iter().filter(|e| e.kind == 1) {
            if event.midi >> 20 & 15 == 9 && event.midi & 127 != 0 {
                self.held += 1;
            } else {
                self.held = self.held.saturating_sub(1);
            }
        }
        self.tone(self.held > 0, out);
        Ok(())
    }
    fn render(&mut self, v: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], _: usize) -> bool {
        self.tone(v.iter().any(|&v| v != 0), out);
        true
    }
}
struct NoEffect;
impl Effect for NoEffect {
    fn apply(&mut self, _: &mut [f32], _: usize) -> bool {
        true
    }
}

const RATE: u32 = 1000;
const FRAMES: usize = 10;
/// Blocks in the one second of silence before a rest.
const QUIET_BLOCKS: usize = RATE as usize / FRAMES;

struct Stream {
    mixer: Mixer,
    renders: Vec<Render<Bell, NoEffect>>,
    command: Command,
    calls: Arc<AtomicUsize>,
}
impl Stream {
    fn new(part: Part, gain: f32) -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let bell = Bell {
            gain,
            level: 0.0,
            held: 0,
            calls: calls.clone(),
        };
        Self {
            mixer: Mixer::with_helpers(RATE, 2, 1, None, 0),
            renders: vec![Render::new(vec![(bell, Layer::default())], None)],
            command: Command {
                generation: 1,
                parts: Arc::new(vec![part]),
                auto_gain_disabled: true,
                ..Command::default()
            },
            calls,
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
    /// One block's output, and whether the instrument ran for it.
    fn block(&mut self) -> (Vec<f32>, bool) {
        let before = self.calls();
        let failed = AtomicBool::new(false);
        let out = self
            .mixer
            .render_block(&mut self.renders, &self.command, FRAMES, &failed)
            .to_vec();
        assert!(!failed.load(Ordering::Relaxed));
        (out, self.calls() > before)
    }
    /// Renders until a block skips the instrument; panics if none does within 10 seconds.
    fn rest(&mut self) {
        for _ in 0..10 * QUIET_BLOCKS {
            if !self.block().1 {
                return;
            }
        }
        panic!("the stream never rested");
    }
}

fn song() -> Part {
    Part {
        sequence: Some(Arc::new(Sequence::new(
            vec![Note {
                start: 0.0,
                duration: 0.05,
                pitch: 60,
                velocity: 100,
            }],
            0.1,
        ))),
        ..Part::default()
    }
}

#[test]
fn the_tail_after_a_song_sounds_unchanged_until_one_second_of_silence_then_rests() {
    const BLOCKS: usize = 300;
    let mut resting = Stream::new(song(), 0.5);
    let mut awake = Stream::new(song(), 0.5);
    awake.mixer.rest.limit = u64::MAX;
    let runs: Vec<_> = (0..BLOCKS).map(|_| resting.block()).collect();
    let reference: Vec<_> = (0..BLOCKS).map(|_| awake.block().0).collect();
    assert!(awake.calls() >= BLOCKS);
    let rendered = runs.iter().take_while(|(_, ran)| *ran).count();
    assert!(runs[rendered..].iter().all(|(_, ran)| !ran));
    let audible = reference
        .iter()
        .rposition(|out| out.iter().any(|s| s.abs() >= rest::THRESHOLD))
        .unwrap();
    // The song ends at block 10 and its release tail lasts past it.
    assert!(audible > 10, "{audible}");
    assert_eq!(rendered, audible + 1 + QUIET_BLOCKS);
    for (block, ((out, _), expected)) in runs.iter().zip(&reference).enumerate() {
        if block < rendered {
            assert_eq!(out, expected, "block {block}");
        } else {
            assert!(out.iter().all(|s| *s == 0.0), "block {block}");
            assert!(expected.iter().all(|s| s.abs() < rest::THRESHOLD));
        }
    }
    assert_eq!(resting.mixer.elapsed, (BLOCKS * FRAMES) as u64);
}

fn live(
    click: &Arc<super::super::click::Click>,
) -> (
    Part,
    super::super::live::Queue,
    Arc<super::super::phrase::Phrase>,
) {
    let queue = super::super::live::Queue::default();
    let phrase = Arc::new(super::super::phrase::Phrase::default());
    (
        Part {
            live: Some(queue.clone()),
            phrase: Some(phrase.clone()),
            click: Some(click.clone()),
            ..Part::default()
        },
        queue,
        phrase,
    )
}

#[test]
fn any_new_input_wakes_a_resting_stream_in_the_same_block() {
    for wake in ["command", "key", "phrase", "metronome"] {
        let click = Arc::new(super::super::click::Click::new());
        let (part, queue, phrase) = live(&click);
        let mut stream = Stream::new(part, 0.5);
        stream.rest();
        assert!(!stream.block().1, "{wake}");
        match wake {
            "command" => stream.command.generation += 1,
            "key" => super::super::live::push(&queue, &[60], 100),
            "phrase" => phrase.play(
                Instant::now(),
                Duration::from_secs(1),
                vec![(Duration::from_secs(2), 60, 100)],
            ),
            _ => click.request(500_000),
        }
        assert!(stream.block().1, "{wake}");
    }
}

#[test]
fn a_held_key_keeps_even_a_silent_instrument_awake_until_released() {
    let click = Arc::new(super::super::click::Click::new());
    let (part, queue, _) = live(&click);
    let mut stream = Stream::new(part, 0.0);
    stream.rest();
    super::super::live::push(&queue, &[60], 100);
    for _ in 0..3 * QUIET_BLOCKS {
        assert!(stream.block().1);
    }
    super::super::live::push(&queue, &[60], 0);
    stream.rest();
}

#[test]
fn audible_output_keeps_the_stream_awake_without_any_input() {
    // A pitch held by the part, as a track audition holds it until stopped.
    let mut held = Part::default();
    held.velocities[60] = 100;
    let mut stream = Stream::new(held, 0.5);
    for _ in 0..3 * QUIET_BLOCKS {
        assert!(stream.block().1);
    }
    // A drone with no input at all: the same pitch held inside the instrument.
    let click = Arc::new(super::super::click::Click::new());
    let (part, queue, _) = live(&click);
    let mut drone = Stream::new(part, 0.5);
    super::super::live::push(&queue, &[60], 100);
    drone.block();
    // Forget the key on the mixer's side, as if its Note On were never counted.
    drone.mixer.lanes[0].held = Default::default();
    for _ in 0..3 * QUIET_BLOCKS {
        assert!(drone.block().1);
    }
}

#[test]
fn the_ui_sees_the_rest_only_while_the_stream_skips_the_latest_command() {
    let click = Arc::new(super::super::click::Click::new());
    let (part, queue, _) = live(&click);
    let mut stream = Stream::new(part, 0.5);
    let resting = stream.mixer.resting.clone();
    stream.block();
    assert!(!resting.on(stream.command.generation));
    stream.rest();
    assert!(resting.on(stream.command.generation));
    // A command the audio thread has not rendered yet already counts as awake.
    stream.command.generation += 1;
    assert!(!resting.on(stream.command.generation));
    assert!(stream.block().1);
    assert!(!resting.on(stream.command.generation));
    stream.rest();
    assert!(resting.on(stream.command.generation));
    super::super::live::push(&queue, &[60], 100);
    assert!(stream.block().1);
    assert!(!resting.on(stream.command.generation));
}
