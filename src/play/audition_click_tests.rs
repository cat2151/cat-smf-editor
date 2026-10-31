use super::super::{mix::Mixer, Command, Part};
use super::*;
use crate::play::layer_render::Render;
use std::sync::Arc;

const RATE: u32 = 44100;
const BLOCK: usize = 1000;

fn command(generation: u64, click: Option<Arc<Click>>) -> Command {
    Command {
        generation,
        parts: Arc::new(vec![Part {
            click,
            ..Part::default()
        }]),
        gains: Arc::new(vec![1.0]),
        auto_gain_disabled: true,
        ..Command::default()
    }
}

/// The left channel of `frames` mixed in blocks, as the output stream would.
fn render(mixer: &mut Mixer, renders: &mut [Render], command: &Command, frames: usize) -> Vec<f32> {
    let failed = AtomicBool::new(false);
    let mut out = vec![];
    while out.len() < frames {
        let block = mixer.block(renders, command, BLOCK, &failed);
        out.extend(block.chunks(2).map(|f| f[0]));
    }
    assert!(!failed.load(Ordering::Relaxed));
    out
}

/// Frames where sound starts after at least 10 ms of silence.
fn onsets(samples: &[f32]) -> Vec<usize> {
    let gap = RATE as usize / 100;
    let mut silent = gap;
    let mut found = vec![];
    for (n, s) in samples.iter().enumerate() {
        if *s == 0.0 {
            silent += 1;
        } else {
            if silent >= gap {
                found.push(n);
            }
            silent = 0;
        }
    }
    found
}

fn crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count()
}

#[test]
fn clicks_fall_on_each_beat_and_the_count_in_ends_on_the_fifth() {
    let click = Arc::new(Click::new());
    let mut renders = vec![Render::new(Vec::new(), None)];
    let mut mixer = Mixer::new(RATE, 2, 1, None);
    let playing = command(1, Some(click.clone()));
    // Silent until requested.
    assert!(render(&mut mixer, &mut renders, &playing, 30_000)
        .iter()
        .all(|s| *s == 0.0));
    assert!(click.origin().is_none());
    click.request(500_000);
    let before = Instant::now();
    let samples = render(&mut mixer, &mut renders, &playing, 22050 * 9 + 2000);
    let after = Instant::now();
    let found = onsets(&samples);
    assert_eq!(found.len(), 10, "{found:?}");
    for (k, &onset) in found.iter().enumerate() {
        assert!(onset.abs_diff(k * 22050) <= 1, "beat {k}: {onset}");
    }
    // The origin is the fifth click, one 4/4 measure after the first.
    let measure = Duration::from_secs(2);
    let origin = click.origin().unwrap();
    assert!(before + measure <= origin, "{:?}", origin - before);
    assert!(origin <= after + measure);
    let first_block = Duration::from_secs_f64(BLOCK as f64 / f64::from(RATE));
    assert!(origin - before <= measure + first_block + Duration::from_millis(50));
    assert_eq!(found[4], 4 * 22050);
    // Measure heads click higher than the other beats.
    let length = RATE as usize * 30 / 1000;
    let tone = |k: usize| crossings(&samples[found[k]..found[k] + length]);
    assert!(tone(0) > tone(1) * 3 / 2, "{} {}", tone(0), tone(1));
    assert_eq!(tone(0), tone(4));
    assert_eq!(tone(1), tone(2));
}

#[test]
fn stopping_the_live_voice_leaves_no_click_and_requests_start_once() {
    let click = Arc::new(Click::new());
    let mut renders = vec![Render::new(Vec::new(), None)];
    let mut mixer = Mixer::new(RATE, 2, 1, None);
    let playing = command(1, Some(click.clone()));
    click.request(500_000);
    let sounded = render(&mut mixer, &mut renders, &playing, 500);
    assert!(sounded.iter().any(|s| *s != 0.0));
    let origin = click.origin().unwrap();
    // A later request keeps the running beat and its origin.
    click.request(250_000);
    render(&mut mixer, &mut renders, &playing, 30_000);
    assert_eq!(click.origin(), Some(origin));
    // Stopping the key voice replaces its command with one that holds no click.
    let stopped = command(2, None);
    assert!(render(&mut mixer, &mut renders, &stopped, 22050 * 5)
        .iter()
        .all(|s| *s == 0.0));
}
