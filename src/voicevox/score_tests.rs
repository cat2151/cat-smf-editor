use super::*;
use crate::model::{Event, Music, Track};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

fn on(tick: u64, key: u8) -> Event {
    event(tick, 0x90, 0, &[key, 100])
}

fn off(tick: u64, key: u8) -> Event {
    event(tick, 0x80, 0, &[key, 0])
}

fn lyric(tick: u64, text: &str) -> Event {
    event(tick, 0xff, 0x05, text.as_bytes())
}

fn tempo(tick: u64, micros: u32) -> Event {
    let b = micros.to_be_bytes();
    event(tick, 0xff, 0x51, &b[1..])
}

fn music(mut events: Vec<Event>) -> Music {
    events.sort_by_key(|e| e.tick);
    Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    }
}

fn frame(seconds: f64) -> u64 {
    LEAD_FRAMES + (seconds * FRAMES_PER_SECOND).round() as u64
}

#[test]
fn rests_lyrics_and_starts_follow_the_track() {
    let m = music(vec![
        lyric(0, "ど"),
        on(0, 60),
        off(480, 60),
        on(960, 64),
        off(1440, 64),
    ]);
    let score = from_track(&m, 0).unwrap();
    let rest = |frames| ScoreNote {
        key: None,
        frame_length: frames,
        lyric: String::new(),
    };
    let note = |key, frames, lyric: &str| ScoreNote {
        key: Some(key),
        frame_length: frames,
        lyric: lyric.to_owned(),
    };
    // 120 BPM: 0.5 s = 46.875 frames → 47, 1.0 s → 94, 1.5 s → 141.
    assert_eq!(
        score.notes,
        vec![
            rest(LEAD_FRAMES),
            note(60, 47, "ど"),
            rest(47),
            note(64, 47, DEFAULT_LYRIC),
            rest(TAIL_FRAMES),
        ]
    );
    assert_eq!(score.starts, vec![0, 15, 62, 109, 156]);
    assert_eq!(score.sources, vec![None, Some(1), None, Some(3), None]);
    assert_eq!(score.total_frames(), LEAD_FRAMES + 141 + TAIL_FRAMES);
    assert!(score.json().starts_with(
        r#"{"notes":[{"key":null,"frame_length":15,"lyric":""},{"key":60,"frame_length":47,"lyric":"ど"}"#
    ));
}

#[test]
fn frames_round_the_cumulative_time_across_tempo_changes() {
    // 160 ticks = 15.625 frames at 120 BPM and 18.75 frames at 100 BPM, so per-note
    // rounding would drift by about half a frame per note.
    let mut events = vec![tempo(0, 500_000), tempo(960, 600_000)];
    for k in 0..30u64 {
        events.push(on(k * 160, 60 + (k % 12) as u8));
        events.push(off(k * 160 + 160, 60 + (k % 12) as u8));
    }
    let m = music(events);
    let score = from_track(&m, 0).unwrap();
    let seconds = |tick: u64| TempoMap::new(&m, 0).duration(0, tick);
    let per_note_rounding: u64 = (0..30)
        .map(|k| (seconds(k * 160 + 160) - seconds(k * 160)) * FRAMES_PER_SECOND)
        .map(|f| f.round() as u64)
        .sum();
    let song = (seconds(30 * 160) * FRAMES_PER_SECOND).round() as u64;
    assert_ne!(per_note_rounding, song);
    assert_eq!(score.total_frames(), LEAD_FRAMES + song + TAIL_FRAMES);
    let sung: Vec<u64> = score
        .sources
        .iter()
        .zip(&score.starts)
        .filter_map(|(source, start)| source.map(|_| *start))
        .collect();
    let expected: Vec<u64> = (0..30).map(|k| frame(seconds(k * 160))).collect();
    assert_eq!(sung, expected);
    assert!(score.notes.iter().all(|n| n.frame_length > 0));
}

#[test]
fn later_notes_cut_earlier_ones_and_chords_sing_the_top() {
    let m = music(vec![
        on(0, 60),
        on(0, 67),
        lyric(0, "き"),
        off(960, 60),
        off(960, 67),
        lyric(480, "ゃ"),
        on(480, 62),
        off(960, 62),
    ]);
    let score = from_track(&m, 0).unwrap();
    let sung: Vec<_> = score
        .notes
        .iter()
        .filter_map(|n| n.key.map(|k| (k, n.frame_length, n.lyric.as_str())))
        .collect();
    assert_eq!(sung, vec![(67, 47, "き"), (62, 47, "ゃ")]);
    assert_eq!(score.notes.len(), 4);
}

#[test]
fn a_track_without_notes_has_no_score() {
    assert_eq!(from_track(&music(vec![lyric(0, "ど")]), 0), None);
    assert_eq!(from_track(&music(vec![]), 0), None);
}

#[test]
fn one_note_sings_ra_between_the_lead_and_tail_rests() {
    let score = one_note(62, 47);
    assert_eq!(score.starts, [0, LEAD_FRAMES, LEAD_FRAMES + 47]);
    assert_eq!(score.total_frames(), LEAD_FRAMES + 47 + TAIL_FRAMES);
    assert_eq!(score.notes[1].key, Some(62));
    assert_eq!(score.notes[1].lyric, DEFAULT_LYRIC);
    assert_eq!(one_note(60, 0).notes[1].frame_length, 1);
}
