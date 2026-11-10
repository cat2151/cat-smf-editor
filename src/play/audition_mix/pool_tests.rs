use super::*;
use cat_smf_editor::instrument_layers::Layer;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

/// A per-part ramp, so mixing parts in another order or twice would change the sum.
struct Ramp {
    part: usize,
    frame: u64,
    calls: Arc<AtomicUsize>,
    threads: Arc<Mutex<Vec<ThreadId>>>,
}
impl Instrument for Ramp {
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], c: usize) -> bool {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.threads
            .lock()
            .unwrap()
            .push(std::thread::current().id());
        // Long enough for every helper to wake and claim a part of its own.
        std::thread::sleep(Duration::from_millis(1));
        for frame in out.chunks_exact_mut(c) {
            let value = (self.part + 1) as f32 * 0.01 * (self.frame % 97) as f32 / 97.0;
            frame.fill(value);
            self.frame += 1;
        }
        true
    }
}
/// Feedback through its own state, so a part's effect running for another part shows.
struct Echo(f32);
impl Effect for Echo {
    fn apply(&mut self, out: &mut [f32], _: usize) -> bool {
        for sample in out {
            *sample += self.0 * 0.5;
            self.0 = *sample;
        }
        true
    }
}

struct Run {
    output: Vec<f32>,
    calls: Vec<usize>,
    threads: usize,
}

fn run(helpers: usize) -> Run {
    const PARTS: usize = 6;
    const BLOCKS: usize = 5;
    let threads = Arc::new(Mutex::new(vec![]));
    let calls: Vec<_> = (0..PARTS).map(|_| Arc::new(AtomicUsize::new(0))).collect();
    let mut renders: Vec<_> = calls
        .iter()
        .enumerate()
        .map(|(part, calls)| {
            Render::new(
                vec![(
                    Ramp {
                        part,
                        frame: 0,
                        calls: calls.clone(),
                        threads: threads.clone(),
                    },
                    Layer::default(),
                )],
                Some(Echo(0.0)),
            )
        })
        .collect();
    let command = Command {
        generation: 1,
        parts: Arc::new((0..PARTS).map(|_| Part::default()).collect()),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let failed = AtomicBool::new(false);
    let mut mixer = Mixer::with_helpers(48000, 2, PARTS, None, helpers);
    assert_eq!(mixer.pool.participants(), helpers + 1);
    let mut output = vec![];
    for block in 0..BLOCKS {
        output.extend_from_slice(mixer.render_block(&mut renders, &command, 100 + block, &failed));
    }
    assert!(!failed.load(Ordering::Relaxed));
    let mut ids = threads.lock().unwrap().clone();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids.dedup();
    Run {
        output,
        calls: calls.iter().map(|c| c.load(Ordering::Relaxed)).collect(),
        threads: ids.len(),
    }
}

#[test]
fn parallel_parts_mix_bit_identically_to_serial_and_each_part_runs_once_per_block() {
    let serial = run(0);
    let parallel = run(3);
    assert_eq!(serial.threads, 1);
    assert!(parallel.threads > 1, "helpers rendered no part");
    assert_eq!(
        serial
            .output
            .iter()
            .map(|s| s.to_bits())
            .collect::<Vec<_>>(),
        parallel
            .output
            .iter()
            .map(|s| s.to_bits())
            .collect::<Vec<_>>()
    );
    assert!(serial.output.iter().any(|s| *s != 0.0));
    assert_eq!(serial.calls, [5; 6]);
    assert_eq!(parallel.calls, [5; 6]);
}

/// Waits for the other part to start, so the two parts are on two threads, then
/// panics if it is the helper.
struct Rendezvous {
    started: Arc<AtomicUsize>,
}
impl Instrument for Rendezvous {
    fn render(&mut self, _: &[u8; 128], _: &[u64; 128], _: i32, out: &mut [f32], _: usize) -> bool {
        self.started.fetch_add(1, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.started.load(Ordering::SeqCst) < 2 {
            assert!(
                Instant::now() < deadline,
                "the parts did not run in parallel"
            );
            std::hint::spin_loop();
        }
        if std::thread::current().name() == Some("audio-part") {
            panic!("helper part fixture");
        }
        out.fill(0.25);
        true
    }
}

#[test]
fn a_panic_in_a_helper_silences_only_its_part_and_reports_failure() {
    let started = Arc::new(AtomicUsize::new(0));
    let mut renders: Vec<_> = (0..2)
        .map(|_| {
            Render::<_, Echo>::new(
                vec![(
                    Rendezvous {
                        started: started.clone(),
                    },
                    Layer::default(),
                )],
                None,
            )
        })
        .collect();
    let command = Command {
        generation: 1,
        parts: Arc::new(vec![Part::default(), Part::default()]),
        auto_gain_disabled: true,
        ..Command::default()
    };
    let failed = AtomicBool::new(false);
    let mut mixer = Mixer::with_helpers(48000, 2, 2, None, 1);
    let output = mixer.render_block(&mut renders, &command, 64, &failed);
    assert_eq!(output, [0.25; 128]);
    assert!(failed.load(Ordering::Relaxed));
}

#[test]
fn dropping_the_mixer_joins_idle_helpers() {
    let mixer = Mixer::with_helpers(48000, 2, 4, None, 3);
    assert_eq!(mixer.pool.participants(), 4);
    drop(mixer);
}

#[test]
fn a_resting_pool_parks_its_spinning_helper_and_the_next_run_still_takes_every_part_once() {
    let mut mixer = Mixer::with_helpers(48000, 2, 6, None, 3);
    let wait = |parked: bool, mixer: &Mixer| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while mixer.pool.parked() != parked {
            assert!(Instant::now() < deadline, "parked never became {parked}");
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    let counts: Vec<_> = (0..6).map(|_| AtomicUsize::new(0)).collect();
    let threads = Mutex::new(vec![]);
    let job = |i: usize| {
        counts[i].fetch_add(1, Ordering::Relaxed);
        threads.lock().unwrap().push(std::thread::current().id());
        std::thread::sleep(Duration::from_millis(1));
    };
    mixer.pool.run(6, &job);
    assert!(!mixer.pool.parked());
    mixer.pool.rest();
    wait(true, &mixer);
    mixer.pool.run(6, &job);
    wait(false, &mixer);
    assert!(counts.iter().all(|c| c.load(Ordering::Relaxed) == 2));
    let mut ids = threads.lock().unwrap().clone();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids.dedup();
    assert!(ids.len() > 1, "helpers rendered no part");
}
