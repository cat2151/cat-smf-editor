use super::{read, regenerate, write, Curve, Point, PREFIX};
use crate::{
    model::{Event, Music, Track},
    score::audition_timing::TempoMap,
};

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// Conductor at 100 BPM with a change at 960; notes in two tracks.
fn song() -> Music {
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    event(0, 0xff, 0x03, b"Conductor"),
                    event(0, 0xff, 0x51, &600_000u32.to_be_bytes()[1..]),
                    event(960, 0xff, 0x51, &400_000u32.to_be_bytes()[1..]),
                    event(960, 0xff, 0x2f, &[]),
                ],
            },
            Track {
                events: vec![
                    event(0, 0x90, 0, &[60, 100]),
                    event(480, 0x80, 0, &[60, 0]),
                    event(960, 0x90, 0, &[62, 100]),
                    event(1440, 0x90, 0, &[62, 0]),
                ],
            },
            Track {
                events: vec![
                    event(720, 0x90, 0, &[48, 100]),
                    event(1200, 0x80, 0, &[48, 0]),
                ],
            },
        ],
    };
    for track in &mut music.tracks {
        crate::edit::refresh_deltas(track).unwrap();
    }
    music
}

fn point(tick: u64, bpm: f64) -> Point {
    Point { tick, bpm }
}

fn curve(points: &[Point], controls: &[Point]) -> Curve {
    Curve {
        version: 1,
        points: points.to_vec(),
        controls: controls.to_vec(),
    }
}

fn tempos(music: &Music) -> Vec<(u64, u32)> {
    music.tracks[0]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == 0x51)
        .map(|e| {
            (
                e.tick,
                u32::from_be_bytes([0, e.data[0], e.data[1], e.data[2]]),
            )
        })
        .collect()
}

fn apply(music: &mut Music, curve: &Curve) {
    write(music, Some(curve)).unwrap();
    regenerate(music, curve).unwrap();
}

#[test]
fn one_point_writes_a_single_tempo_at_tick_zero() {
    let mut music = song();
    apply(&mut music, &curve(&[point(480, 120.0)], &[]));
    assert_eq!(tempos(&music), vec![(0, 500_000)]);
    let deltas_match = music.tracks[0]
        .events
        .windows(2)
        .all(|w| u64::from(w[1].delta) == w[1].tick - w[0].tick);
    assert!(deltas_match);
}

#[test]
fn a_straight_segment_writes_linear_tempos_where_notes_start_or_end() {
    let mut music = song();
    // 120 BPM at 240 down to 60 BPM at 1200; control at the midpoint.
    apply(
        &mut music,
        &curve(&[point(240, 120.0), point(1200, 60.0)], &[point(720, 90.0)]),
    );
    let linear = |tick: f64| {
        let bpm = 120.0 - 60.0 * (tick.clamp(240.0, 1200.0) - 240.0) / 960.0;
        (60_000_000.0 / bpm).round() as u32
    };
    // Note ticks: 0, 480, 720, 960, 1200, 1440. 1440 equals 1200's value and is skipped.
    let expected: Vec<_> = [0u64, 480, 720, 960, 1200]
        .into_iter()
        .map(|t| (t, linear(t as f64)))
        .collect();
    assert_eq!(tempos(&music), expected);
    assert_eq!(expected[0].1, 500_000);
    assert_eq!(expected[4].1, 1_000_000);
}

#[test]
fn moving_the_control_bends_the_inside_and_keeps_the_ends() {
    let ends = [point(240, 120.0), point(1200, 60.0)];
    let straight = curve(&ends, &[point(720, 90.0)]);
    let up = curve(&ends, &[point(720, 150.0)]);
    let down = curve(&ends, &[point(720, 40.0)]);
    let left = curve(&ends, &[point(300, 90.0)]);
    let right = curve(&ends, &[point(1150, 90.0)]);
    for bent in [&up, &down, &left, &right] {
        assert_eq!(bent.bpm_at(240.0), 120.0);
        assert_eq!(bent.bpm_at(1200.0), 60.0);
        assert_eq!(bent.bpm_at(0.0), 120.0);
        assert_eq!(bent.bpm_at(5000.0), 60.0);
        for tick in [480.0, 720.0, 960.0] {
            let difference = (bent.bpm_at(tick) - straight.bpm_at(tick)).abs();
            assert!(difference > 1.0, "{bent:?} at {tick}");
        }
    }
    assert!(up.bpm_at(720.0) > straight.bpm_at(720.0));
    assert!(down.bpm_at(720.0) < straight.bpm_at(720.0));
    // A control pulled left reaches the lower tempo sooner.
    assert!(left.bpm_at(720.0) < straight.bpm_at(720.0));
    assert!(right.bpm_at(720.0) > straight.bpm_at(720.0));
    assert!((straight.bpm_at(480.0) - 105.0).abs() < 1e-9);
}

#[test]
fn a_flat_120_bpm_curve_times_notes_like_a_plain_500000_tempo() {
    let mut curved = song();
    apply(&mut curved, &curve(&[point(0, 120.0)], &[]));
    let mut plain = song();
    plain.tracks[0]
        .events
        .retain(|e| !(e.status == 0xff && e.meta_type == 0x51));
    crate::edit::insert(
        &mut plain.tracks[0],
        event(0, 0xff, 0x51, &500_000u32.to_be_bytes()[1..]),
    );
    let ticks: Vec<_> = curved.tracks[1..]
        .iter()
        .flat_map(|t| &t.events)
        .map(|e| e.tick)
        .collect();
    for track in 1..3 {
        let (a, b) = (TempoMap::new(&curved, track), TempoMap::new(&plain, track));
        for &tick in &ticks {
            assert_eq!(a.duration(0, tick), b.duration(0, tick));
        }
    }
    assert!((TempoMap::new(&curved, 1).duration(0, 1440) - 1.5).abs() < 1e-12);
}

#[test]
fn metadata_round_trips_and_keeps_notes_names_and_master_effects() {
    let mut music = song();
    let master = crate::master_effects::Settings {
        effects: vec![crate::chain_settings::PluginState {
            reference: None,
            plugin: crate::host::PluginKey {
                format: "CLAP".into(),
                id: "effect".into(),
                name: "Effect".into(),
                vendor: "fixture".into(),
                bundle_path: "X:/plugins/effect.clap".into(),
            },
            state: vec![1],
            preset_name: None,
            bypass: true,
        }],
        ..Default::default()
    };
    crate::master_effects::write(&mut music, &master).unwrap();
    let notes = format!("{:?}", music.tracks[1..].to_vec());
    let saved = curve(
        &[point(0, 120.5), point(960, 80.25), point(1440, 200.0)],
        &[point(400, 130.0), point(1440, 90.0)],
    );
    write(&mut music, Some(&saved)).unwrap();
    // Replacing keeps a single metadata event.
    write(&mut music, Some(&saved)).unwrap();
    assert_eq!(read(&music).unwrap(), Some(saved.clone()));
    let reread = crate::ffi::parse(&crate::smf_io::encode(&music).unwrap()).unwrap();
    assert_eq!(read(&reread).unwrap(), Some(saved));
    assert_eq!(crate::master_effects::read(&reread).unwrap(), master);
    assert_eq!(format!("{:?}", music.tracks[1..].to_vec()), notes);
    assert_eq!(music.tracks[0].name().as_deref(), Some("Conductor"));
    write(&mut music, None).unwrap();
    assert_eq!(read(&music).unwrap(), None);
    assert_eq!(crate::master_effects::read(&music).unwrap(), master);
    assert_eq!(tempos(&music), tempos(&song()));
}

fn with_meta(track: usize, tick: u64, json: &str) -> Music {
    let mut music = song();
    let mut data = PREFIX.to_vec();
    data.extend(json.as_bytes());
    crate::edit::insert(&mut music.tracks[track], event(tick, 0xff, 0x7f, &data));
    music
}

#[test]
fn malformed_metadata_is_an_error() {
    let good = r#"{"version":1,"points":[{"tick":0,"bpm":120.0}],"controls":[]}"#;
    assert!(read(&with_meta(0, 0, good)).unwrap().is_some());
    let mut duplicate = with_meta(0, 0, good);
    let copy = duplicate.tracks[0]
        .events
        .iter()
        .find(|e| super::owned(e))
        .unwrap()
        .clone();
    duplicate.tracks[0].events.insert(0, copy);
    let mut format0 = with_meta(0, 0, good);
    format0.format = 0;
    let cases = [
        duplicate,
        with_meta(1, 0, good),
        with_meta(0, 1, good),
        format0,
        with_meta(0, 0, &good.replace("\"version\":1", "\"version\":2")),
        with_meta(
            0,
            0,
            r#"{"version":1,"points":[{"tick":0,"bpm":120.0}],"controls":[{"tick":0,"bpm":120.0}]}"#,
        ),
        with_meta(
            0,
            0,
            r#"{"version":1,"points":[{"tick":480,"bpm":120.0},{"tick":0,"bpm":90.0}],"controls":[{"tick":240,"bpm":100.0}]}"#,
        ),
        with_meta(0, 0, &good.replace("120.0", "0.0")),
        with_meta(0, 0, "{"),
    ];
    for (i, music) in cases.iter().enumerate() {
        assert!(read(music).is_err(), "case {i}");
    }
}
