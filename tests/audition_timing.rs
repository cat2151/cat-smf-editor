use cat_smf_editor::{
    model::{Event, Music, Track},
    score::audition_timing::duration_seconds,
};
fn tempo(tick: u64, value: u32) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x51,
        data: vec![(value >> 16) as u8, (value >> 8) as u8, value as u8],
    }
}
#[test]
fn integrates_tempo_changes_and_uses_source_track_for_format_two() {
    let mut m = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![tempo(0, 500_000), tempo(480, 1_000_000)],
            },
            Track {
                events: vec![tempo(0, 250_000)],
            },
        ],
    };
    assert_eq!(duration_seconds(&m, 1, 240, 720), 1.125);
    m.format = 2;
    assert_eq!(duration_seconds(&m, 1, 240, 720), 0.375);
    assert_eq!(duration_seconds(&m, 0, 240, 720), 1.25);
    assert_eq!(duration_seconds(&m, 0, 480, 0), 0.0);
}
#[test]
fn smpte_drop_frame_is_independent_of_tempo() {
    let m = Music {
        format: 1,
        division: 0xe350,
        tracks: vec![],
    };
    assert!((duration_seconds(&m, 0, 123, 2400) - 1.001).abs() < 1e-12);
}

#[test]
fn inverse_tempo_mapping_crosses_changes_from_nonzero_start_and_smpte() {
    use cat_smf_editor::score::audition_timing::TempoMap;
    let music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![tempo(480, 1_000_000)],
        }],
    };
    let map = TempoMap::new(&music, 0);
    assert_eq!(map.tick_after(0, 0.25), 240);
    assert_eq!(map.tick_after(0, 0.5), 480);
    assert_eq!(map.tick_after(240, 0.75), 720);
    let music = Music {
        format: 1,
        division: 0xe350,
        tracks: vec![],
    };
    assert_eq!(TempoMap::new(&music, 0).tick_after(123, 1.001), 2523);
}
