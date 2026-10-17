use cat_smf_editor::{
    model::{Event, Music, Track},
    score::musical_time::MusicalTime,
};

#[test]
fn time_columns_measure_entire_track_and_space_pad_without_changing_time() {
    let music = Music {
        format: 1,
        division: 480,
        tracks: vec![Track {
            events: vec![
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![61, 100],
                },
                Event {
                    delta: 9,
                    tick: 9,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![72, 100],
                },
                Event {
                    delta: 90,
                    tick: 99,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![0, 0],
                },
                Event {
                    delta: 380,
                    tick: 479,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            ],
        }],
    };
    let time = MusicalTime::new(&music);
    let map = time.map(0).unwrap();
    let columns = super::TimeColumns::new(&music.tracks[0].events, Some(map));
    assert_eq!(columns.position(&map.position(0)), "1:1:  0");
    assert_eq!(columns.position(&map.position(9)), "1:1:  9");
    assert_eq!(columns.position(&map.position(479)), "1:1:479");
    assert_eq!(columns.pitch_range, Some((61, 72)));
    assert_eq!((columns.delta, columns.absolute), (3, 3));
}

#[test]
fn time_columns_handle_fractional_ticks_empty_and_single_pitch_tracks() {
    let mut music = Music {
        format: 1,
        division: 3,
        tracks: vec![Track {
            events: vec![
                Event {
                    delta: 0,
                    tick: 0,
                    status: 0xff,
                    meta_type: 0x58,
                    data: vec![4, 3, 24, 8],
                },
                Event {
                    delta: 2,
                    tick: 2,
                    status: 0x90,
                    meta_type: 0,
                    data: vec![127, 100],
                },
            ],
        }],
    };
    let time = MusicalTime::new(&music);
    let map = time.map(0).unwrap();
    let columns = super::TimeColumns::new(&music.tracks[0].events, Some(map));
    assert_eq!(columns.position(&map.position(0)), "1:1:  0");
    assert_eq!(columns.position(&map.position(2)), "1:2:1/2");
    assert_eq!(columns.rest_position(&map.position(2), false), "1:2");
    assert_eq!(columns.rest_position(&map.position(2), true), "1");
    assert_eq!(columns.pitch_range, Some((127, 127)));
    music.tracks[0].events.clear();
    let columns = super::TimeColumns::new(&[], None);
    assert_eq!(columns.pitch_range, None);
    assert_eq!(columns.position_chars(), 5);
}
