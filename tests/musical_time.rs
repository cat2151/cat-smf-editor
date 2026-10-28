use cat_smf_editor::{
    model::{Event, Music, Track},
    score::musical_time::MusicalTime,
};

fn meter(tick: u64, numerator: u8, exponent: u8) -> Event {
    Event {
        delta: 0,
        tick,
        status: 0xff,
        meta_type: 0x58,
        data: vec![numerator, exponent, 24, 8],
    }
}
fn music(format: u8, division: u16, tracks: Vec<Vec<Event>>) -> Music {
    Music {
        format,
        division,
        tracks: tracks.into_iter().map(|events| Track { events }).collect(),
    }
}
fn positions(events: Vec<Event>, expected: &[(u64, &str)]) {
    let m = music(1, 480, vec![events]);
    let time = MusicalTime::new(&m);
    for &(tick, expected) in expected {
        assert_eq!(time.map(0).unwrap().position(tick).to_string(), expected);
    }
}

#[test]
fn agreed_positions_and_meter_boundaries() {
    positions(vec![], &[(0, "1:1:0"), (480, "1:2:0"), (1920, "2:1:0")]);
    positions(vec![meter(0, 6, 3)], &[(240, "1:2:0"), (1440, "2:1:0")]);
    positions(
        vec![meter(960, 3, 2)],
        &[(959, "1:2:479"), (960, "2:1:0"), (2400, "3:1:0")],
    );
    positions(vec![meter(1920, 3, 2)], &[(1920, "2:1:0")]);
    positions(vec![meter(960, 4, 2)], &[(960, "1:3:0"), (1920, "2:1:0")]);
    positions(vec![meter(0, 3, 2)], &[(0, "1:1:0"), (1440, "2:1:0")]);
}

#[test]
fn global_conductor_ties_and_format_two() {
    let m = music(1, 480, vec![vec![meter(0, 6, 3)], vec![meter(0, 3, 2)]]);
    let time = MusicalTime::new(&m);
    assert_eq!(time.map(1).unwrap().position(480).to_string(), "1:2:0");
    assert_eq!(time.map(0).unwrap().warnings.len(), 1);
    let m = music(2, 480, m.tracks.into_iter().map(|t| t.events).collect());
    let time = MusicalTime::new(&m);
    assert_eq!(time.map(0).unwrap().position(480).to_string(), "1:3:0");
    assert_eq!(time.map(1).unwrap().position(480).to_string(), "1:2:0");
    assert!(time.map(0).unwrap().warnings.is_empty());
    let m = music(1, 480, vec![vec![meter(0, 6, 3), meter(0, 2, 2)], vec![]]);
    let time = MusicalTime::new(&m);
    assert_eq!(time.map(1).unwrap().position(960).to_string(), "2:1:0");
}

#[test]
fn defaults_smpte_invalid_and_exact_fractions() {
    let mut bad = meter(0, 3, 2);
    bad.data.pop();
    let m = music(1, 480, vec![vec![bad, meter(0, 0, 2), meter(0, 4, 255)]]);
    let time = MusicalTime::new(&m);
    let map = time.map(0).unwrap();
    assert_eq!(map.warnings.len(), 3);
    assert_eq!(map.position(480).to_string(), "1:2:0");
    assert!(MusicalTime::new(&music(1, 0xe728, vec![vec![]]))
        .map(0)
        .is_none());
    assert!(MusicalTime::new(&music(1, 0, vec![vec![]]))
        .map(0)
        .is_none());
    let time = MusicalTime::new(&music(1, 3, vec![vec![meter(0, 4, 3)]]));
    assert_eq!(time.map(0).unwrap().position(2).to_string(), "1:2:1/2");
    let time = MusicalTime::new(&music(
        1,
        1,
        vec![vec![meter(0, 1, 63), meter(u64::MAX - 1, 2, 63)]],
    ));
    assert_eq!(time.map(0).unwrap().position(u64::MAX - 1).beat, 1);
    assert!(time.map(0).unwrap().position(u64::MAX).measure > u64::MAX.into());
}

#[test]
fn tempo_does_not_change_positions() {
    let tempo = Event {
        delta: 0,
        tick: 123,
        status: 0xff,
        meta_type: 0x51,
        data: vec![7, 0xa1, 0x20],
    };
    positions(vec![tempo], &[(1920, "2:1:0")]);
}
