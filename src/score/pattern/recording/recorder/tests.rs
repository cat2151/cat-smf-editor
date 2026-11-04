use super::*;
use crate::{
    edit,
    score::pattern::{
        body,
        recording::{Key, Mode, Modulation, Velocity},
        Catalog,
    },
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn settings(range: Range) -> Settings {
    let mut s = Settings::new(Key::from_signature(None), 500_000);
    s.range = range;
    // CC1 has its own test; these compare notes only.
    s.modulation = Modulation::Off;
    s
}
fn z(column: u8) -> Pad {
    Pad::Row { row: 0, column }
}
/// (pattern-relative tick, status, pitch).
type Notes = Vec<(u64, u8, u8)>;

/// Bar count and notes of each committed pattern, in creation order.
fn bodies(m: &Music, ids: &[u64]) -> Vec<(usize, Notes)> {
    let c = Catalog::read(m).unwrap();
    ids.iter()
        .map(|id| {
            let p = &c.patterns[id];
            let b = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
            (
                p.meters.len(),
                b.iter()
                    .map(|e| (e.tick - p.tick, e.status, e.data[0]))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn shared_chord_notes_sound_until_the_last_pad_releases() {
    let mut s = settings(Range::Free);
    s.mode = Mode::Triad;
    let mut r = Recorder::new(480, s).unwrap();
    // C major triads from 48: column 0 = C E G, column 2 = E G B.
    r.start(ms(0));
    assert_eq!(r.press(z(0), ms(0)).unwrap(), vec![48, 52, 55]);
    assert_eq!(r.press(z(2), ms(500)).unwrap(), vec![59]);
    assert_eq!(r.release(z(0), ms(1000)).unwrap(), vec![48]);
    assert!(r.sounding(52) && r.sounding(55));
    assert_eq!(r.release(z(2), ms(1500)).unwrap(), vec![52, 55, 59]);
    assert!((0..128).all(|p| !r.sounding(p)));
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    let mut notes = bodies(&m, &ids).remove(0).1;
    notes.sort_by_key(|&(tick, status, pitch)| (pitch, tick, status));
    assert_eq!(
        notes,
        vec![
            (0, 0x90, 48),
            (960, 0x80, 48),
            (0, 0x90, 52),
            (1440, 0x80, 52),
            (0, 0x90, 55),
            (1440, 0x80, 55),
            (480, 0x90, 59),
            (1440, 0x80, 59),
        ]
    );
}

#[test]
fn releasing_stops_the_notes_pressed_even_after_octave_or_mode_change() {
    let mut r = Recorder::new(480, settings(Range::Free)).unwrap();
    assert_eq!(r.press(z(0), ms(0)).unwrap(), vec![60]);
    r.settings_mut().note_base = 72;
    assert_eq!(r.release(z(0), ms(100)).unwrap(), vec![60]);
    assert_eq!(r.press(z(0), ms(200)).unwrap(), vec![72]);
    r.settings_mut().mode = Mode::Triad;
    assert_eq!(r.release(z(0), ms(300)).unwrap(), vec![72]);
    assert_eq!(r.press(z(0), ms(400)).unwrap(), vec![48, 52, 55]);
    r.settings_mut().mode = Mode::Note;
    assert_eq!(r.release_all(ms(500)).unwrap(), vec![48, 52, 55]);
    assert!((0..128).all(|p| !r.sounding(p)));
}

#[test]
fn loop_laps_become_takes_split_at_the_boundary_and_round_trip() {
    // 2 bars at 120 BPM = 4 s = 3840 ticks per lap.
    let mut r = Recorder::new(480, settings(Range::Loop { measures: 2 })).unwrap();
    r.start(ms(1000));
    r.press(z(0), ms(1000)).unwrap(); // lap 0, tick 0
    r.release(z(0), ms(2000)).unwrap();
    r.press(z(1), ms(4000)).unwrap(); // lap 0, tick 2880, held into lap 1
    r.release(z(1), ms(6000)).unwrap(); // lap 1, tick 960
    r.press(z(2), ms(9500)).unwrap(); // lap 2, tick 480
    r.release(z(2), ms(10000)).unwrap();
    r.press(z(3), ms(14000)).unwrap(); // lap 3, tick 960; 3.5 laps end at 15 s
    r.release_all(ms(15000)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    assert_eq!(ids.len(), 4);
    assert_eq!(
        bodies(&m, &ids),
        vec![
            (
                2,
                vec![
                    (0, 0x90, 60),
                    (960, 0x80, 60),
                    (2880, 0x90, 62),
                    (3840, 0x80, 62)
                ]
            ),
            (2, vec![(0, 0x90, 62), (960, 0x80, 62)]),
            (2, vec![(480, 0x90, 64), (960, 0x80, 64)]),
            (2, vec![(960, 0x90, 67), (1920, 0x80, 67)]),
        ]
    );
    assert_eq!(
        Catalog::read(&m).unwrap().sources.len(),
        1,
        "all takes share one container"
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("loop.mid");
    std::fs::write(&path, crate::smf_io::encode(&m).unwrap()).unwrap();
    let reopened = crate::smf_io::load(&path).unwrap();
    assert_eq!(
        crate::smf_io::encode(&m).unwrap(),
        crate::smf_io::encode(&reopened).unwrap()
    );
    assert_eq!(bodies(&reopened, &ids), bodies(&m, &ids));
}

#[test]
fn empty_laps_and_count_in_presses_are_not_recorded() {
    let mut r = Recorder::new(480, settings(Range::Loop { measures: 1 })).unwrap();
    assert!(!r.started());
    // Count-in presses sound but are not recorded.
    assert_eq!(r.press(z(4), ms(0)).unwrap(), vec![69]);
    assert_eq!(r.release(z(4), ms(500)).unwrap(), vec![69]);
    r.start(ms(2000));
    r.press(z(0), ms(2000)).unwrap(); // lap 0
    r.release(z(0), ms(2500)).unwrap();
    r.press(z(1), ms(6500)).unwrap(); // lap 2; lap 1 stays empty
    r.release(z(1), ms(7000)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    assert_eq!(
        bodies(&m, &ids),
        vec![
            (1, vec![(0, 0x90, 60), (480, 0x80, 60)]),
            (1, vec![(480, 0x90, 62), (960, 0x80, 62)]),
        ]
    );
    let mut silent = Recorder::new(480, settings(Range::Loop { measures: 1 })).unwrap();
    silent.start(ms(0));
    assert!(silent
        .commit(&mut edit::tracks::new_song(), None, Duration::ZERO)
        .is_err());
}

#[test]
fn a_release_on_the_lap_boundary_ends_the_earlier_lap() {
    let mut r = Recorder::new(480, settings(Range::Loop { measures: 1 })).unwrap();
    r.start(ms(0));
    r.press(z(0), ms(1000)).unwrap();
    r.release(z(0), ms(2000)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    assert_eq!(
        bodies(&m, &ids),
        vec![(1, vec![(960, 0x90, 60), (1920, 0x80, 60)])]
    );
}

#[test]
fn free_take_is_measured_from_rec_and_held_notes_block_commit() {
    let mut r = Recorder::new(480, settings(Range::Free)).unwrap();
    r.start(ms(9000));
    r.press(z(0), ms(9000)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    assert!(r.commit(&mut m.clone(), None, Duration::ZERO).is_err());
    r.release(z(0), ms(11250)).unwrap();
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    assert_eq!(
        bodies(&m, &ids),
        vec![(2, vec![(0, 0x90, 60), (2160, 0x80, 60)])]
    );
    assert!(Recorder::new(480, settings(Range::Loop { measures: 0 })).is_err());
    assert!(Recorder::new(0xe728, settings(Range::Free)).is_err());
}

#[test]
fn takes_hold_each_press_velocity_and_the_cc1_played_over_each_lap() {
    // 1 bar at 120 BPM = 2 s = 1920 ticks per lap.
    let mut s = settings(Range::Loop { measures: 1 });
    s.velocity = Velocity::Range80To127;
    s.modulation = Modulation::Sweep;
    let mut r = Recorder::new(480, s).unwrap();
    r.start(ms(1000));
    r.press(z(0), ms(1000)).unwrap();
    assert_eq!(r.velocity(), 80);
    r.release(z(0), ms(1500)).unwrap();
    r.press(z(1), ms(2500)).unwrap(); // held into lap 1
    assert_eq!(r.velocity(), 86);
    r.release_all(ms(3500)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    let c = Catalog::read(&m).unwrap();
    for (lap, id) in ids.iter().enumerate() {
        let p = &c.patterns[id];
        let b = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
        let notes: Vec<_> = b
            .iter()
            .filter(|e| e.status == 0x90)
            .map(|e| (e.data[0], e.data[1]))
            .collect();
        let controls: Vec<_> = b.iter().filter(|e| e.status == 0xb0).collect();
        // The lap restart of the held note keeps its velocity.
        let expected = if lap == 0 {
            vec![(60, 80), (62, 86)]
        } else {
            vec![(62, 86)]
        };
        assert_eq!(notes, expected);
        // The session-clock value at the lap start, then one per sweep step, inside the lap.
        let start = ms(1000 + 2000 * lap as u64);
        assert_eq!(controls[0].tick, p.tick);
        assert_eq!(
            controls[0].data,
            vec![1, Modulation::Sweep.value(start).unwrap()]
        );
        // A step a tick apart from the last still gets its own tick.
        assert!(controls.len() > 120, "lap {lap}");
        assert!(controls.windows(2).all(|w| w[0].tick < w[1].tick));
        assert!(controls.iter().all(|e| e.tick < p.tick + 1920));
    }
}

#[test]
fn bpm_90_puts_one_beat_on_division_ticks() {
    let mut s = settings(Range::Free);
    s.tempo = 60_000_000 / 90;
    let mut r = Recorder::new(480, s).unwrap();
    r.start(ms(0));
    r.press(z(0), ms(0)).unwrap();
    let beat = Duration::from_secs(60) / 90;
    r.press(z(1), beat).unwrap();
    r.release_all(beat * 2).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    let notes = bodies(&m, &ids).remove(0).1;
    assert!(notes.contains(&(480, 0x90, 62)), "{notes:?}");
}

#[test]
fn a_quantized_take_starts_and_ends_its_notes_on_sixteenths() {
    let mut s = settings(Range::Free);
    s.quantize = true;
    let mut r = Recorder::new(480, s).unwrap();
    // 10 ms is 10 ticks and 260 ms is 250 ticks at 120 BPM.
    r.start(ms(0));
    r.press(z(0), ms(0)).unwrap();
    r.release(z(0), ms(10)).unwrap();
    r.press(z(1), ms(140)).unwrap();
    r.release(z(1), ms(260)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let ids = r.commit(&mut m, None, Duration::ZERO).unwrap();
    assert_eq!(
        bodies(&m, &ids),
        vec![(
            1,
            vec![
                (0, 0x90, 60),
                (120, 0x80, 60),
                (120, 0x90, 62),
                (240, 0x80, 62)
            ]
        )]
    );
}
