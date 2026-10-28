use super::*;
use crate::model::Track;

fn note(tick: u64, status: u8, pitch: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, 71],
    }
}
fn music(mut events: Vec<Event>) -> Music {
    events.sort_by_key(|e| e.tick);
    Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events }],
    }
}

#[test]
fn same_measure_is_transient_preserves_channel_and_roundtrips() {
    let mut m = music(vec![
        note(120, 0x93, 62),
        note(600, 0x83, 62),
        event(3840, 1),
    ]);
    let before = crate::smf_io::encode(&m).unwrap();
    assert_eq!(
        crate::score::event_content::decoded(&event(3840, 1)),
        "Same measure: 1"
    );
    let (events, end) = expand(&m, 0).unwrap();
    assert_eq!(end, 5760);
    let copied: Vec<_> = events
        .iter()
        .filter(|e| e.tick >= 3840 && e.status != 0xff)
        .collect();
    assert_eq!(
        copied
            .iter()
            .map(|e| (e.tick, e.status, e.data[0], e.data[1]))
            .collect::<Vec<_>>(),
        vec![(3960, 0x93, 62, 71), (4440, 0x83, 62, 0)]
    );
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    let parsed = crate::ffi::parse(&before).unwrap();
    assert_eq!(
        reference(parsed.tracks[0].events.iter().find(|e| owned(e)).unwrap()).unwrap(),
        Some(1)
    );
    m.tracks[0].events[0].data[0] = 65;
    m.tracks[0].events[1].data[0] = 65;
    assert!(expand(&m, 0)
        .unwrap()
        .0
        .iter()
        .any(|e| e.tick == 3960 && e.data[0] == 65));
}

#[test]
fn same_measure_rejects_invalid_sources_destinations_and_payloads() {
    for events in [
        vec![event(0, 1)],
        vec![event(1920, 1), event(3840, 2)],
        vec![note(0, 0x90, 60), note(1921, 0x80, 60), event(3840, 1)],
        vec![note(1800, 0x90, 60), note(2000, 0x80, 60), event(3840, 2)],
        vec![note(3840, 0x90, 60), note(4000, 0x80, 60), event(3840, 1)],
        vec![note(1800, 0x90, 60), note(2000, 0x80, 60), event(1920, 3)],
        vec![event(1921, 1)],
        vec![event(1920, 1), event(1920, 3)],
        vec![event(1920, 0)],
    ] {
        assert!(expand(&music(events), 0).is_err());
    }
    let mut bad = event(1920, 1);
    bad.data.pop();
    assert!(expand(&music(vec![bad]), 0).is_err());
}

#[test]
fn same_measure_empty_beyond_eot_and_variable_meter() {
    assert_eq!(expand(&music(vec![event(1920, 50)]), 0).unwrap().1, 3840);
    let meter = |tick, beats, exponent| Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x58,
        data: vec![beats, exponent, 24, 8],
    };
    assert!(expand(&music(vec![meter(1920, 8, 3), event(1920, 1)]), 0).is_ok());
    assert!(expand(&music(vec![meter(1920, 3, 2), event(1920, 1)]), 0).is_err());
}

#[test]
fn same_measure_batch_is_atomic_on_late_collision_and_preserves_non_notes() {
    let mut m = music(vec![note(5800, 0x90, 60), note(5900, 0x80, 60)]);
    let before = crate::smf_io::encode(&m).unwrap();
    assert!(insert(&mut m, 0, 1, 2, 3).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    assert!(insert(&mut m, 0, 1, 2, 2).is_err());
    assert_eq!(crate::smf_io::encode(&m).unwrap(), before);
    let cc = Event {
        tick: 3840,
        delta: 0,
        status: 0xb0,
        meta_type: 0,
        data: vec![1, 64],
    };
    let mut m = music(vec![cc]);
    insert(&mut m, 0, 1, 2, 3).unwrap();
    assert_eq!(m.tracks[0].events[0].data, vec![1, 64]);
    assert_eq!(m.tracks[0].events.iter().filter(|e| owned(e)).count(), 2);
}

#[test]
fn extracted_polyphony_is_in_tick_order_with_stable_retriggers_and_zero_length_notes() {
    let m = music(vec![
        note(0, 0x93, 60),
        note(0, 0x93, 64),
        note(120, 0x93, 67),
        note(240, 0x83, 60),
        note(240, 0x93, 60),
        note(240, 0x93, 72),
        note(240, 0x83, 72),
        note(360, 0x83, 60),
        note(480, 0x83, 64),
        note(600, 0x83, 67),
    ]);
    let mut body = Track {
        events: notes(&m.tracks[0].events, (0, 1920)).unwrap(),
    };
    crate::edit::refresh_deltas(&mut body).unwrap();
    assert_eq!(
        body.events
            .iter()
            .map(|e| (e.tick, e.status, e.data[0]))
            .collect::<Vec<_>>(),
        vec![
            (0, 0x93, 60),
            (0, 0x93, 64),
            (120, 0x93, 67),
            (240, 0x83, 60),
            (240, 0x93, 60),
            (240, 0x93, 72),
            (240, 0x83, 72),
            (360, 0x83, 60),
            (480, 0x83, 64),
            (600, 0x83, 67),
        ]
    );
}

fn meter(tick: u64, beats: u8, exponent: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x58,
        data: vec![beats, exponent, 24, 8],
    }
}

fn error(events: Vec<Event>) -> String {
    expand(&music(events), 0).unwrap_err()
}

#[test]
fn expand_reports_each_error_with_track_tick_and_message() {
    let crossing = "Note crosses measure boundary";
    for (events, expected) in [
        (
            vec![event(1921, 1)],
            "tick 1921: Reference must be at measure start".to_string(),
        ),
        (vec![event(0, 1)], "tick 0: Self reference".into()),
        (
            vec![meter(1920, 3, 2), event(1920, 1)],
            "tick 1920: Measure lengths differ".into(),
        ),
        (
            vec![event(1920, 1), event(3840, 2)],
            "tick 3840: Chained same measure reference".into(),
        ),
        (
            vec![event(1920, 1), event(1920, 3)],
            "tick 1920: Multiple references in destination".into(),
        ),
        (
            vec![note(3840, 0x90, 60), note(4000, 0x80, 60), event(3840, 1)],
            "tick 3840: Destination contains notes".into(),
        ),
        (
            vec![note(0, 0x90, 60), note(1921, 0x80, 60), event(3840, 1)],
            format!("tick 3840: {crossing}"),
        ),
        (
            vec![note(1800, 0x90, 60), note(2000, 0x80, 60), event(3840, 2)],
            format!("tick 3840: {crossing}"),
        ),
        (
            vec![note(1800, 0x90, 60), note(2000, 0x80, 60), event(1920, 3)],
            format!("tick 1920: {crossing}"),
        ),
        (
            vec![note(100, 0x90, 60), event(3840, 1)],
            format!("tick 3840: {crossing}"),
        ),
        (
            vec![note(100, 0x90, 60), event(1920, 3)],
            format!("tick 1920: {crossing}"),
        ),
        (
            vec![note(5000, 0x90, 60), note(6000, 0x80, 60), event(3840, 1)],
            format!("tick 3840: {crossing}"),
        ),
        (
            vec![event(1920, 0)],
            "tick 1920: Same measure number must be positive".into(),
        ),
        (
            vec![event(1921, 1), event(3840, 2)],
            "tick 1921: Reference must be at measure start".into(),
        ),
        (
            vec![event(1920, 1), event(5760, 2), event(5761, 1)],
            "tick 5760: Chained same measure reference".into(),
        ),
    ] {
        assert_eq!(error(events), format!("Track 1, {expected}"));
    }
    let mut bad = event(1920, 1);
    bad.data.pop();
    assert_eq!(
        error(vec![bad]),
        "Track 1, tick 1920: Invalid same measure payload"
    );
    assert_eq!(expand(&music(vec![]), 1).unwrap_err(), "Missing track");
    let mut smpte = music(vec![event(1920, 1)]);
    smpte.division = 0xe728;
    assert_eq!(
        expand(&smpte, 0).unwrap_err(),
        "Same measure requires PPQN timing"
    );
}

#[test]
fn expand_copies_every_reference_in_order_with_end() {
    let cc = Event {
        tick: 100,
        delta: 0,
        status: 0xb2,
        meta_type: 0,
        data: vec![1, 64],
    };
    let mut silent_on = note(960, 0x92, 60);
    silent_on.data[1] = 0;
    let m = music(vec![
        note(0, 0x92, 60),
        note(0, 0x92, 64),
        cc,
        note(240, 0x82, 60),
        note(240, 0x92, 60),
        note(240, 0x92, 67),
        note(240, 0x92, 67),
        note(480, 0x82, 67),
        note(480, 0x82, 67),
        note(480, 0x82, 64),
        silent_on,
        note(1000, 0x92, 74),
        note(1000, 0x82, 74),
        note(1900, 0x92, 72),
        note(1920, 0x82, 72),
        note(3000, 0x95, 50),
        note(3840, 0x85, 50),
        event(3840, 1),
        event(5760, 1),
        note(7700, 0x90, 40),
        note(7700, 0x90, 41),
        note(8000, 0x80, 41),
        note(8000, 0x80, 40),
        event(9600, 5),
        note(11600, 0x91, 30),
    ]);
    let (events, end) = expand(&m, 0).unwrap();
    assert_eq!(end, 11600);
    let summary: Vec<_> = events
        .iter()
        .map(|e| {
            (
                e.tick,
                e.status,
                e.data.first().copied(),
                e.data.get(1).copied(),
            )
        })
        .collect();
    assert_eq!(format!("{summary:?}"), EXPANDED);
}

const EXPANDED: &str = "[(0, 146, Some(60), Some(71)), (0, 146, Some(64), Some(71)), (100, 178, Some(1), Some(64)), \
    (240, 130, Some(60), Some(71)), (240, 146, Some(60), Some(71)), (240, 146, Some(67), Some(71)), \
    (240, 146, Some(67), Some(71)), (480, 130, Some(67), Some(71)), (480, 130, Some(67), Some(71)), \
    (480, 130, Some(64), Some(71)), (960, 146, Some(60), Some(0)), (1000, 146, Some(74), Some(71)), \
    (1000, 130, Some(74), Some(71)), (1900, 146, Some(72), Some(71)), (1920, 130, Some(72), Some(71)), \
    (3000, 149, Some(50), Some(71)), (3840, 133, Some(50), Some(71)), (3840, 255, Some(125), Some(99)), \
    (3840, 146, Some(60), Some(71)), (3840, 146, Some(64), Some(71)), (4080, 130, Some(60), Some(0)), \
    (4080, 146, Some(60), Some(71)), (4080, 146, Some(67), Some(71)), (4080, 146, Some(67), Some(71)), \
    (4320, 130, Some(64), Some(0)), (4320, 130, Some(67), Some(0)), (4320, 130, Some(67), Some(0)), \
    (4800, 130, Some(60), Some(0)), (4840, 146, Some(74), Some(71)), (4840, 130, Some(74), Some(0)), \
    (5740, 146, Some(72), Some(71)), (5760, 255, Some(125), Some(99)), (5760, 130, Some(72), Some(0)), \
    (5760, 146, Some(60), Some(71)), (5760, 146, Some(64), Some(71)), (6000, 130, Some(60), Some(0)), \
    (6000, 146, Some(60), Some(71)), (6000, 146, Some(67), Some(71)), (6000, 146, Some(67), Some(71)), \
    (6240, 130, Some(64), Some(0)), (6240, 130, Some(67), Some(0)), (6240, 130, Some(67), Some(0)), \
    (6720, 130, Some(60), Some(0)), (6760, 146, Some(74), Some(71)), (6760, 130, Some(74), Some(0)), \
    (7660, 146, Some(72), Some(71)), (7680, 130, Some(72), Some(0)), (7700, 144, Some(40), Some(71)), \
    (7700, 144, Some(41), Some(71)), (8000, 128, Some(41), Some(71)), (8000, 128, Some(40), Some(71)), \
    (9600, 255, Some(125), Some(99)), (9620, 144, Some(40), Some(71)), (9620, 144, Some(41), Some(71)), \
    (9920, 128, Some(40), Some(0)), (9920, 128, Some(41), Some(0)), (11600, 145, Some(30), Some(71))]";
