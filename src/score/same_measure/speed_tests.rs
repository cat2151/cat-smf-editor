use super::*;
use crate::model::Track;
use std::time::{Duration, Instant};

/// Even tracks reference measure 2 from every odd measure; odd tracks hold notes.
fn referencing_song(tracks: usize, measures: u64) -> Music {
    let note = |tick, status, pitch| Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, 80],
    };
    let tracks = (0..tracks)
        .map(|t| {
            let mut events = Vec::new();
            for m in 0..measures {
                let start = m * 1920;
                if t % 2 == 0 && m % 2 == 0 {
                    events.push(event(start, 2));
                    continue;
                }
                for beat in 0..8 {
                    let tick = start + beat * 240;
                    events.push(note(tick, 0x90, 60 + beat as u8));
                    events.push(note(tick + 200, 0x80, 60 + beat as u8));
                }
            }
            events.sort_by_key(|e| e.tick);
            Track { events }
        })
        .collect();
    Music {
        format: 1,
        division: 480,
        tracks,
    }
}

#[test]
#[ignore = "timing; run with cargo test --release -- --ignored"]
fn expanding_a_referencing_song_fits_well_inside_a_frame() {
    let music = referencing_song(16, 200);
    let mut best = Duration::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        for track in 0..music.tracks.len() {
            let (events, _) = expand(&music, track).unwrap();
            assert!(!events.is_empty());
        }
        best = best.min(started.elapsed());
    }
    assert!(best < Duration::from_millis(5), "{best:?}");
}
