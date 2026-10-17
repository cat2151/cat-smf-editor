use super::*;
use crate::model::Track;

fn song() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    };
    for (tick, micros) in [
        (0, 500_000u32),
        (480, 600_000),
        (960, 500_000),
        (1440, 400_000),
    ] {
        music.tracks[0].events.push(Event {
            tick,
            delta: 0,
            status: 0xff,
            meta_type: 0x51,
            data: micros.to_be_bytes()[1..].to_vec(),
        });
    }
    refresh_deltas(&mut music.tracks[0]).unwrap();
    music
}

fn other_events(music: &Music) -> String {
    format!(
        "{:?}",
        music.tracks[0]
            .events
            .iter()
            .filter(|e| !owned(e))
            .map(|e| (e.tick, e.status, e.meta_type, &e.data))
            .collect::<Vec<_>>()
    )
}

#[test]
fn captures_original_tempos_and_tied_ranks_once_and_round_trips() {
    let mut music = song();
    let before = other_events(&music);
    initialize(&mut music).unwrap();
    let points = read(&music).unwrap();
    assert_eq!(
        points.iter().map(|p| p.rank).collect::<Vec<_>>(),
        [2, 3, 2, 1]
    );
    assert_eq!(ranks(&points), [2, 3, 2, 1]);
    assert_eq!(other_events(&music), before);
    set(&mut music, 480, 160.0).unwrap();
    let moved = read(&music).unwrap();
    assert_eq!(moved[1].rank, 3);
    assert_eq!(ranks(&moved), [3, 1, 3, 2]);
    initialize(&mut music).unwrap();
    assert_eq!(read(&music).unwrap(), moved);
    assert_eq!(other_events(&music), before);
    let bytes = crate::smf_io::encode(&music).unwrap();
    let reloaded = crate::ffi::parse(&bytes).unwrap();
    assert_eq!(read(&reloaded).unwrap(), moved);
    set(&mut music, 480, 100.0).unwrap();
    assert_eq!(ranks(&read(&music).unwrap()), [2, 3, 2, 1]);
}

#[test]
fn default_tempo_and_duplicate_tick_use_effective_tempo() {
    let mut music = song();
    music.tracks[0].events.clear();
    initialize(&mut music).unwrap();
    assert_eq!(
        read(&music).unwrap(),
        [Reference {
            tick: 0,
            bpm: 120.0,
            rank: 1
        }]
    );
    let mut music = song();
    music.tracks[0].events[0].tick = 480;
    initialize(&mut music).unwrap();
    let points = read(&music).unwrap();
    assert_eq!(points[0].bpm, 120.0);
    assert_eq!(points[1].bpm, 100.0);
}

#[test]
fn guide_survives_curve_generation_and_removal() {
    use crate::edit::tempo_curve::{self, Curve, Point};
    let mut music = song();
    initialize(&mut music).unwrap();
    let points = read(&music).unwrap();
    let curve = Curve {
        version: 1,
        points: vec![Point { tick: 0, bpm: 80.0 }],
        controls: vec![],
    };
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut music, &curve).unwrap();
    tempo_curve::write(&mut music, None).unwrap();
    initialize(&mut music).unwrap();
    assert_eq!(read(&music).unwrap(), points);
}

#[test]
fn rejects_invalid_metadata_and_bpm_without_overwriting() {
    let mut music = song();
    initialize(&mut music).unwrap();
    let before = format!("{music:?}");
    for bpm in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(set(&mut music, 0, bpm).is_err());
        assert_eq!(format!("{music:?}"), before);
    }
    assert!(set(&mut music, 123, 120.0).is_err());
    let i = music.tracks[0].events.iter().position(owned).unwrap();
    music.tracks[0].events[i].data = b"Reference Tempo: 120 BPM; rank=0".to_vec();
    assert!(initialize(&mut music).is_err());
    music.tracks[0].events[i].data = b"Reference Tempo: NaN BPM; rank=1".to_vec();
    assert!(read(&music).is_err());
}
