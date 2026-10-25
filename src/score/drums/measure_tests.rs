use super::*;
use crate::{
    model::Music,
    score::{musical_time::MusicalTime, note_view::NoteView},
};

fn song(numerator: u8, exponent: u8) -> Music {
    let mut track = Track {
        events: vec![Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![numerator, exponent, 24, 8],
        }],
    };
    write(&mut track, Some(&Settings::default())).unwrap();
    Music {
        format: 0,
        division: 480,
        tracks: vec![track],
    }
}

#[test]
fn actual_measures_follow_meter_changes_and_fractional_ppqn() {
    for (n, d, steps, length) in [(4, 2, 16, 1920), (3, 2, 12, 1440), (6, 3, 12, 1440)] {
        let music = song(n, d);
        let time = MusicalTime::new(&music);
        let bar = Measure::new(time.map(0).unwrap(), 480, 3).unwrap();
        assert_eq!(
            (bar.number, bar.start, bar.end, bar.steps.len()),
            (3, 2 * length, 3 * length, steps)
        );
    }
    let mut music = song(4, 2);
    edit::insert(
        &mut music.tracks[0],
        Event {
            tick: 1920,
            delta: 0,
            status: 0xff,
            meta_type: 0x58,
            data: vec![3, 2, 24, 8],
        },
    );
    let time = MusicalTime::new(&music);
    assert_eq!(
        Measure::new(time.map(0).unwrap(), 480, 2)
            .unwrap()
            .steps
            .len(),
        12
    );
    music.division = 5;
    let time = MusicalTime::new(&music);
    let bar = Measure::new(time.map(0).unwrap(), 5, 1).unwrap();
    assert_eq!(bar.steps.len(), 16);
    assert!(bar.steps.windows(2).all(|w| w[0] < w[1]));
    let mut music = song(3, 3);
    music.division = 5;
    let time = MusicalTime::new(&music);
    assert_eq!(
        Measure::new(time.map(0).unwrap(), 5, 1).unwrap().steps,
        vec![0, 2, 3, 4, 5, 7]
    );
    assert_eq!(
        Measure::new(time.map(0).unwrap(), 5, 2).unwrap().steps,
        vec![8, 9, 10, 12, 13, 14]
    );
}

#[test]
fn adding_in_reverse_and_deleting_extends_each_row_to_next_hit_or_bar_end() {
    let mut music = song(3, 2);
    for step in [11, 8, 4, 0] {
        toggle_measure(&mut music, 0, 0, 2, step).unwrap();
    }
    toggle_measure(&mut music, 0, 1, 2, 2).unwrap();
    let view = NoteView::new(&music.tracks[0].events);
    let notes: Vec<_> = music.tracks[0]
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .map(|(i, e)| (e.tick, e.data[0], view.durations[i].unwrap()))
        .collect();
    assert_eq!(
        notes,
        vec![
            (1440, 36, 480),
            (1680, 38, 1200),
            (1920, 36, 480),
            (2400, 36, 360),
            (2760, 36, 120)
        ]
    );
    toggle_measure(&mut music, 0, 0, 2, 4).unwrap();
    let view = NoteView::new(&music.tracks[0].events);
    let i = music.tracks[0]
        .events
        .iter()
        .position(|e| note_on(e) && e.tick == 1440)
        .unwrap();
    assert_eq!(view.durations[i], Some(960));
    let bytes = crate::smf_io::encode(&music).unwrap();
    let restored = crate::ffi::parse(&bytes).unwrap();
    assert_eq!(crate::smf_io::encode(&restored).unwrap(), bytes);
}
