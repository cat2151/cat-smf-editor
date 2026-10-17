use super::*;
use crate::{
    edit::{
        self,
        tempo_curve::{self, Point},
        trill,
    },
    score::audition_timing::TempoMap,
};

fn managed() -> Music {
    let mut music = trill::tests::song();
    trill::initialize(&mut music).unwrap();
    music
}

fn assert_timing(music: &Music) {
    for r in trill::read(music).unwrap() {
        let map = TempoMap::new(music, r.track);
        let ons: Vec<_> = music.tracks[r.track]
            .events
            .iter()
            .filter(|e| {
                note_on(e)
                    && r.start <= e.tick
                    && e.tick < r.end
                    && e.status & 15 == r.metadata.channel - 1
                    && r.metadata.notes.contains(&e.data[0])
            })
            .collect();
        assert!(!ons.is_empty());
        for (i, e) in ons.iter().enumerate() {
            let actual = map.duration(r.start, e.tick - r.start);
            let target = r.metadata.onset_seconds(i);
            // One tick is the unavoidable SMF quantization limit. Use the exact
            // local tick duration from the exported FF51 events, not curve BPM.
            let tick_seconds = map.duration(e.tick.saturating_sub(1), 2);
            assert!(
                (actual - target).abs() <= tick_seconds / 2.0 + 1e-9,
                "{}: onset {i} actual={actual} target={target}",
                r.metadata.id
            );
            if i > 0 {
                assert!(e.tick > ons[i - 1].tick);
                assert_ne!(e.data[0], ons[i - 1].data[0]);
            }
        }
        let view = crate::score::note_view::NoteView::new(&music.tracks[r.track].events);
        for (i, e) in music.tracks[r.track]
            .events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                note_on(e)
                    && r.start <= e.tick
                    && e.tick < r.end
                    && r.metadata.notes.contains(&e.data[0])
            })
        {
            assert!(view.durations[i].is_some_and(|d| d > 0 && e.tick + d <= r.end));
        }
    }
}

#[test]
fn reflows_across_tempo_changes_and_is_idempotent_after_save_and_reload() {
    let mut music = managed();
    let others: Vec<_> = music.tracks[1]
        .events
        .iter()
        .filter(|e| e.data.first() == Some(&48))
        .map(|e| (e.tick, e.status, e.data.clone()))
        .collect();
    edit::regenerate(&mut music).unwrap();
    assert_timing(&music);
    assert_eq!(
        music.tracks[1]
            .events
            .iter()
            .filter(|e| note_on(e) && e.data[0] == 60)
            .count(),
        6
    );
    let bytes = crate::smf_io::encode(&music).unwrap();
    let mut decoded = crate::ffi::parse(&bytes).unwrap();
    edit::regenerate(&mut decoded).unwrap();
    assert_eq!(crate::smf_io::encode(&decoded).unwrap(), bytes);
    assert_eq!(
        music.tracks[1]
            .events
            .iter()
            .filter(|e| e.data.first() == Some(&48))
            .map(|e| (e.tick, e.status, e.data.clone()))
            .collect::<Vec<_>>(),
        others
    );
}

#[test]
fn curved_tempo_is_solved_with_generated_note_boundaries_and_retains_speed_when_changed() {
    let mut music = managed();
    let mut curve = Curve {
        version: 1,
        points: vec![
            Point { tick: 0, bpm: 30.0 },
            Point {
                tick: 400,
                bpm: 240.0,
            },
        ],
        controls: vec![Point {
            tick: 200,
            bpm: 35.0,
        }],
    };
    for bpm in [240.0, 45.0, 300.0] {
        curve.points[1].bpm = bpm;
        tempo_curve::write(&mut music, Some(&curve)).unwrap();
        edit::regenerate(&mut music).unwrap();
        assert_timing(&music);
        let bytes = crate::smf_io::encode(&music).unwrap();
        edit::regenerate(&mut music).unwrap();
        assert_eq!(crate::smf_io::encode(&music).unwrap(), bytes);
        assert!(
            (trill::read(&music).unwrap()[0].metadata.interval_seconds - 0.05625).abs() < 1e-12
        );
    }
}

#[test]
fn interval_edit_can_leave_one_note_then_restore_both_pitches_and_initial_phase() {
    let mut music = managed();
    edit::regenerate(&mut music).unwrap();
    trill::set_interval(&mut music, 1, 1, 5.0).unwrap();
    edit::regenerate(&mut music).unwrap();
    let count = |music: &Music| {
        music.tracks[1]
            .events
            .iter()
            .filter(|e| note_on(e) && [60, 62].contains(&e.data[0]))
            .count()
    };
    assert_eq!(count(&music), 1);
    trill::set_interval(&mut music, 1, 1, 0.02).unwrap();
    edit::regenerate(&mut music).unwrap();
    assert!(count(&music) > 8);
    assert_timing(&music);
    assert_eq!(
        music.tracks[1]
            .events
            .iter()
            .find(|e| note_on(e) && [60, 62].contains(&e.data[0]))
            .unwrap()
            .data,
        [60, 90]
    );
}

#[test]
fn impossible_speed_and_boundary_collisions_leave_music_unchanged() {
    for case in 0..4 {
        let mut music = managed();
        match case {
            0 => trill::set_interval(&mut music, 1, 1, 1e-12).unwrap(),
            1 => {
                let mut on = music.tracks[1]
                    .events
                    .iter()
                    .find(|e| note_on(e) && e.data[0] == 60)
                    .unwrap()
                    .clone();
                on.tick = 90;
                music.tracks[1].events.insert(0, on);
            }
            2 => {
                let on = music.tracks[1]
                    .events
                    .iter()
                    .find(|e| note_on(e) && e.data[0] == 60)
                    .unwrap()
                    .clone();
                music.tracks[1].events.insert(1, on);
            }
            _ => music.tracks[1].events.push(Event {
                tick: 292,
                delta: 0,
                status: 0x80,
                meta_type: 0,
                data: vec![60, 0],
            }),
        }
        music.tracks[1].events.sort_by_key(|e| e.tick);
        let before = format!("{music:?}");
        assert!(edit::regenerate(&mut music).is_err(), "case {case}");
        assert_eq!(format!("{music:?}"), before);
    }
    let mut music = managed();
    for invalid in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(trill::set_interval(&mut music, 1, 1, invalid).is_err());
    }
}

#[test]
fn generated_off_precedes_following_same_pitch_on_at_region_end() {
    let mut music = managed();
    music.tracks[1].events.extend([
        Event {
            tick: 292,
            delta: 0,
            status: 0x90,
            meta_type: 0,
            data: vec![60, 77],
        },
        Event {
            tick: 350,
            delta: 0,
            status: 0x80,
            meta_type: 0,
            data: vec![60, 0],
        },
    ]);
    music.tracks[1].events.sort_by_key(|e| e.tick);
    edit::regenerate(&mut music).unwrap();
    let events = &music.tracks[1].events;
    let i = events
        .iter()
        .position(|e| e.tick == 292 && note_on(e) && e.data[0] == 60)
        .unwrap();
    assert!(events[..i]
        .iter()
        .any(|e| e.tick == 292 && note_off(e) && e.data[0] == 60));
    let view = crate::score::note_view::NoteView::new(events);
    assert_eq!(view.durations[i], Some(58));
}

#[test]
fn first_note_has_independent_duration_across_curves_and_interval_edits() {
    let mut music = managed();
    trill::set_timing(&mut music, 1, 1, 0.04, 0.12).unwrap();
    for bpm in [45.0, 180.0] {
        tempo_curve::write(
            &mut music,
            Some(&Curve {
                version: 1,
                points: vec![
                    Point { tick: 0, bpm },
                    Point {
                        tick: 400,
                        bpm: 240.0,
                    },
                ],
                controls: vec![Point {
                    tick: 200,
                    bpm: 60.0,
                }],
            }),
        )
        .unwrap();
        edit::regenerate(&mut music).unwrap();
        assert_timing(&music);
        let bytes = crate::smf_io::encode(&music).unwrap();
        music = crate::ffi::parse(&bytes).unwrap();
        edit::regenerate(&mut music).unwrap();
        assert_eq!(crate::smf_io::encode(&music).unwrap(), bytes);
    }
    trill::set_interval(&mut music, 1, 1, 0.06).unwrap();
    edit::regenerate(&mut music).unwrap();
    assert_timing(&music);
    assert_eq!(
        trill::read(&music).unwrap()[0].metadata.first_note_seconds,
        Some(0.12)
    );
    // A first note longer than the region is clipped at the end; it can be
    // shortened later without losing the original alternating pattern.
    trill::set_timing(&mut music, 1, 1, 0.04, 2.0).unwrap();
    edit::regenerate(&mut music).unwrap();
    assert_eq!(
        music.tracks[1]
            .events
            .iter()
            .filter(|e| note_on(e) && [60, 62].contains(&e.data[0]))
            .count(),
        1
    );
    for invalid in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        let before = format!("{music:?}");
        assert!(trill::set_timing(&mut music, 1, 1, 0.04, invalid).is_err());
        assert_eq!(format!("{music:?}"), before);
    }
}
