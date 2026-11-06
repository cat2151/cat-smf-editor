use super::*;
use crate::play::audio_sequence::Note;

#[test]
fn kit_swing_uses_actual_note_identity_and_the_same_exact_shuffle_timing() {
    let tempo = TempoMap::constant(480, 120.0).unwrap();
    let kit = cat_smf_editor::drum_kit::Kit {
        layer: 1,
        notes: vec![],
        rows: None,
        muted: vec![],
        swing: [(
            127,
            cat_smf_editor::drum_kit::Swing {
                enabled: true,
                amount: 66,
            },
        )]
        .into(),
    };
    let notes = [0, 127]
        .into_iter()
        .map(|pitch| Note {
            pitch,
            velocity: 83,
            start: 0.125,
            duration: 0.125,
        })
        .collect();
    let sequence = Sequence::new(notes, 0.5).kit_swing(Some(&kit), 480, &tempo);
    let straight = sequence.notes.iter().find(|n| n.pitch == 0).unwrap();
    let swung = sequence.notes.iter().find(|n| n.pitch == 127).unwrap();
    assert_eq!((straight.start, straight.duration), (0.125, 0.125));
    assert!((swung.start - 1.0 / 6.0).abs() < 1e-10);
    assert!((swung.start + swung.duration - 0.25).abs() < 1e-10);
    assert!(sequence
        .events
        .iter()
        .filter(|e| (e.word >> 8) & 127 == 127 && (e.word >> 20) & 15 == 9)
        .all(|e| (e.time - swung.start).abs() < 1e-10));
}

#[test]
fn rows_swing_and_velocity_are_independent_and_shuffle_is_exact() {
    let tempo = TempoMap::constant(480, 120.0).unwrap();
    let mut settings = Settings::default();
    settings.rows[0].swing_enabled = true;
    settings.rows[0].swing = 66;
    settings.rows[0].velocity = 71;
    settings.rows[1].velocity = 92;
    let notes = [36, 38]
        .into_iter()
        .map(|pitch| Note {
            pitch,
            velocity: if pitch == 36 { 71 } else { 92 },
            start: 0.125,
            duration: 0.125,
        })
        .collect();
    let sequence = Sequence::new(notes, 0.5).drums(Some(&settings), 480, &tempo);
    let kick = sequence.notes.iter().find(|n| n.pitch == 36).unwrap();
    let snare = sequence.notes.iter().find(|n| n.pitch == 38).unwrap();
    assert!((kick.start - 1.0 / 6.0).abs() < 1e-10);
    assert!((kick.start + kick.duration - 0.25).abs() < 1e-10);
    assert_eq!(kick.velocity, 71);
    assert_eq!(
        (snare.start, snare.duration, snare.velocity),
        (0.125, 0.125, 92)
    );
    let kick_on = sequence
        .events
        .iter()
        .find(|e| e.word & 0xff00 == 36 << 8 && (e.word >> 20) & 15 == 9)
        .unwrap();
    assert_eq!(kick_on.word & 127, 71);
    assert!((kick_on.time - kick.start).abs() < 1e-10);
    assert!(sequence.events.windows(2).all(|e| e[0].time <= e[1].time));
}

#[test]
fn swing_extremes_straight_disabled_and_fractional_ppqn_keep_pair_boundaries() {
    let mut row = Settings::default().rows[0].clone();
    for division in [4, 101, 480] {
        let pair = f64::from(division) / 2.0;
        for value in [1, 50, 66, 99] {
            row.swing = value;
            row.swing_enabled = true;
            let mut previous = -1.0;
            for n in 0..=200 {
                let tick = pair * n as f64 / 100.0;
                let warped = row.swung_tick(tick, division);
                assert!(warped > previous);
                previous = warped;
                if n % 100 == 0 {
                    assert!((warped - tick).abs() < 1e-10);
                }
                if value == 50 {
                    assert_eq!(warped, tick);
                }
            }
            row.swing_enabled = false;
            assert_eq!(row.swung_tick(pair * 0.5, division), pair * 0.5);
        }
    }
}

#[test]
fn swing_uses_tick_time_across_a_tempo_change_and_closes_at_loop_end() {
    use cat_smf_editor::model::{Event, Music, Track};
    let music = Music {
        format: 0,
        division: 480,
        tracks: vec![Track {
            events: vec![Event {
                tick: 120,
                delta: 120,
                status: 0xff,
                meta_type: 0x51,
                data: vec![0x0f, 0x42, 0x40],
            }],
        }],
    };
    let tempo = TempoMap::new(&music, 0);
    let mut settings = Settings::default();
    settings.rows[0].swing_enabled = true;
    settings.rows[0].swing = 66;
    let sequence = Sequence::new(
        vec![Note {
            pitch: 36,
            velocity: 100,
            start: 0.125,
            duration: 0.25,
        }],
        0.375,
    )
    .drums(Some(&settings), 480, &tempo);
    assert!((sequence.notes[0].start - tempo.seconds_at(160)).abs() < 1e-10);
    assert!((sequence.events.last().unwrap().time - 0.375).abs() < 1e-10);
}
