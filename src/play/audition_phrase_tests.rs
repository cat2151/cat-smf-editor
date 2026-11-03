use super::*;
use crate::play::audition::live::word;

const MS: Duration = Duration::from_millis(1);

fn due(p: &Phrase, now: Instant) -> Vec<u32> {
    let mut out = [0; 64];
    let n = p.due(now, &mut out);
    out[..n].to_vec()
}

/// One note held for half of a 1000 ms lap, starting 100 ms in.
fn phrase(start: Instant) -> Phrase {
    let p = Phrase::default();
    p.play(
        start,
        1000 * MS,
        vec![(600 * MS, 60, 0), (100 * MS, 60, 80)],
    );
    p
}

#[test]
fn every_lap_replays_the_phrase_once() {
    let start = Instant::now();
    let p = phrase(start);
    assert!(due(&p, start).is_empty());
    assert_eq!(due(&p, start + 101 * MS), vec![word(60, 80)]);
    assert!(due(&p, start + 500 * MS).is_empty());
    assert_eq!(due(&p, start + 700 * MS), vec![word(60, 0)]);
    assert_eq!(
        due(&p, start + 2200 * MS),
        vec![word(60, 80), word(60, 0), word(60, 80)]
    );
}

#[test]
fn an_off_at_the_lap_end_precedes_the_next_laps_on() {
    let start = Instant::now();
    let p = Phrase::default();
    p.play(
        start,
        1000 * MS,
        vec![(Duration::ZERO, 60, 80), (1000 * MS, 60, 0)],
    );
    assert_eq!(due(&p, start + 500 * MS), vec![word(60, 80)]);
    assert_eq!(due(&p, start + 1500 * MS), vec![word(60, 0), word(60, 80)]);
}

#[test]
fn a_late_start_catches_up_without_repeating() {
    let start = Instant::now();
    let p = phrase(start);
    assert_eq!(due(&p, start + 300 * MS), vec![word(60, 80)]);
    assert!(due(&p, start + 300 * MS).is_empty());
}

#[test]
fn stop_reports_the_notes_left_sounding() {
    let start = Instant::now();
    let p = phrase(start);
    due(&p, start + 200 * MS);
    assert!(p.playing());
    assert_eq!(p.stop(), vec![60]);
    assert!(!p.playing());
    assert!(due(&p, start + 2000 * MS).is_empty());
    let p = phrase(start);
    due(&p, start + 700 * MS);
    assert!(p.stop().is_empty());
}

#[test]
fn a_full_block_resumes_where_it_stopped() {
    let start = Instant::now();
    let p = Phrase::default();
    p.play(
        start,
        1000 * MS,
        (0..3).map(|i| (i * 10 * MS, 60 + i as u8, 80)).collect(),
    );
    let mut out = [0; 2];
    assert_eq!(p.due(start + 100 * MS, &mut out), 2);
    assert_eq!(due(&p, start + 100 * MS), vec![word(62, 80)]);
}

#[test]
fn a_replaced_phrase_starts_at_the_next_lap() {
    let start = Instant::now();
    let p = Phrase::default();
    // Held across the lap end, so the old phrase leaves 60 sounding there.
    p.play(
        start,
        1000 * MS,
        vec![(100 * MS, 60, 80), (1000 * MS, 60, 0)],
    );
    assert_eq!(due(&p, start + 200 * MS), vec![word(60, 80)]);
    p.replace(vec![(300 * MS, 64, 0), (Duration::ZERO, 64, 90)]);
    assert!(due(&p, start + 900 * MS).is_empty(), "the lap plays out");
    assert_eq!(due(&p, start + 1100 * MS), vec![word(60, 0), word(64, 90)]);
    assert_eq!(due(&p, start + 2100 * MS), vec![word(64, 0), word(64, 90)]);
}

#[test]
fn a_replacement_waits_for_room_in_the_block() {
    let start = Instant::now();
    let p = Phrase::default();
    p.play(
        start,
        1000 * MS,
        vec![(Duration::ZERO, 60, 80), (Duration::ZERO, 62, 80)],
    );
    assert_eq!(due(&p, start + 100 * MS).len(), 2);
    p.replace(vec![(Duration::ZERO, 64, 90)]);
    let mut out = [0; 1];
    assert_eq!(p.due(start + 1100 * MS, &mut out), 0);
    assert_eq!(
        due(&p, start + 1100 * MS),
        vec![word(60, 0), word(62, 0), word(64, 90)]
    );
}

#[test]
fn a_replacement_from_a_later_lap_lets_the_laps_before_it_play() {
    let start = Instant::now();
    let p = Phrase::default();
    p.play(start, 1000 * MS, vec![(Duration::ZERO, 60, 80)]);
    p.replace_from(2000 * MS, vec![(Duration::ZERO, 64, 90)]);
    assert_eq!(due(&p, start + 1100 * MS), vec![word(60, 80), word(60, 80)]);
    assert_eq!(due(&p, start + 2100 * MS), vec![word(60, 0), word(64, 90)]);
}
