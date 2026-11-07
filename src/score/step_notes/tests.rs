use super::*;

const C4: u8 = 60;
const STEP: u64 = 120;

/// A conductor in `numerator`/4 and an empty note track (track 1), 480 ticks per quarter.
fn song(numerator: u8) -> Music {
    let meta = Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x58,
        data: vec![numerator, 2, 24, 8],
    };
    Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![meta] }, Track { events: vec![] }],
    }
}
fn bar(numerator: u8) -> u64 {
    u64::from(numerator) * 480
}
/// (start, end, velocity) of every note of `pitch` on track 1.
fn spans(music: &Music, pitch: u8) -> Vec<(u64, u64, u8)> {
    notes(&music.tracks[1])
        .into_iter()
        .filter(|n| n.pitch == pitch)
        .map(|n| (n.start, n.end, n.velocity))
        .collect()
}
fn row(music: &Music, number: u64, pitch: u8) -> Vec<Cell> {
    let time = MusicalTime::new(music);
    let map = time.map(1).unwrap();
    let measure = Measure::new(map, music.division, number).unwrap();
    cells(
        &notes(&music.tracks[1]),
        map,
        music.division,
        &measure,
        pitch,
    )
}
fn grow(music: &mut Music, number: u64, step: usize, limit: u64) {
    resize(music, 1, C4, number, step, Length::Longer, limit).unwrap();
}
fn shrink(music: &mut Music, number: u64, step: usize) {
    resize(music, 1, C4, number, step, Length::Shorter, u64::MAX).unwrap();
}
const HEAD: Cell = Cell::Head { aligned: true };
const CONT: Cell = Cell::Continuation { aligned: true };

#[test]
fn step_notes_toggle_places_and_removes_one_step_with_its_own_note_off() {
    for n in [4, 3] {
        let mut m = song(n);
        toggle(&mut m, 1, C4, 1, 2, 100).unwrap();
        let events: Vec<_> = m.tracks[1]
            .events
            .iter()
            .filter(|e| e.status & 0xf0 == 0x90 || e.status & 0xf0 == 0x80)
            .map(|e| (e.tick, e.status, e.data.clone()))
            .collect();
        assert_eq!(
            events,
            [(240, 0x90, vec![C4, 100]), (360, 0x80, vec![C4, 0])]
        );
        let mut expected = vec![Cell::Empty; usize::from(n) * 4];
        expected[2] = HEAD;
        assert_eq!(row(&m, 1, C4), expected);
        assert_eq!(row(&m, 1, C4 + 1), vec![Cell::Empty; usize::from(n) * 4]);
        toggle(&mut m, 1, C4, 1, 2, 100).unwrap();
        assert_eq!(spans(&m, C4), []);
    }
}

#[test]
fn step_notes_toggle_on_a_continuation_removes_the_whole_note() {
    let mut m = song(4);
    toggle(&mut m, 1, C4, 1, 0, 100).unwrap();
    grow(&mut m, 1, 0, u64::MAX);
    grow(&mut m, 1, 0, u64::MAX);
    assert_eq!(spans(&m, C4), [(0, 3 * STEP, 100)]);
    toggle(&mut m, 1, C4, 1, 2, 100).unwrap();
    assert_eq!(spans(&m, C4), []);
}

#[test]
fn step_notes_adjacent_same_pitch_notes_stay_separate() {
    let mut m = song(4);
    toggle(&mut m, 1, C4, 1, 0, 100).unwrap();
    toggle(&mut m, 1, C4, 1, 1, 100).unwrap();
    assert_eq!(spans(&m, C4), [(0, STEP, 100), (STEP, 2 * STEP, 100)]);
    // The first note's Note Off precedes the second note's Note On at the shared tick.
    let at: Vec<_> = m.tracks[1]
        .events
        .iter()
        .filter(|e| e.tick == STEP)
        .map(|e| e.status & 0xf0)
        .collect();
    assert_eq!(at, [0x80, 0x90]);
    assert_eq!(&row(&m, 1, C4)[..3], [HEAD, HEAD, Cell::Empty]);
}

#[test]
fn step_notes_extending_stops_at_the_next_head_of_the_same_pitch() {
    let mut m = song(4);
    toggle(&mut m, 1, C4, 1, 0, 100).unwrap();
    toggle(&mut m, 1, C4, 1, 2, 100).unwrap();
    toggle(&mut m, 1, C4 + 1, 1, 1, 100).unwrap();
    grow(&mut m, 1, 0, u64::MAX);
    assert_eq!(
        spans(&m, C4),
        [(0, 2 * STEP, 100), (2 * STEP, 3 * STEP, 100)]
    );
    grow(&mut m, 1, 1, u64::MAX);
    assert_eq!(
        spans(&m, C4),
        [(0, 2 * STEP, 100), (2 * STEP, 3 * STEP, 100)]
    );
    assert_eq!(&row(&m, 1, C4)[..4], [HEAD, CONT, HEAD, Cell::Empty]);
}

#[test]
fn step_notes_extend_across_the_bar_and_shrink_from_the_next_bar() {
    for n in [4, 3] {
        let mut m = song(n);
        let last = usize::from(n) * 4 - 1;
        toggle(&mut m, 1, C4, 1, last, 100).unwrap();
        grow(&mut m, 1, last, u64::MAX);
        grow(&mut m, 2, 0, u64::MAX);
        let start = bar(n) - STEP;
        assert_eq!(spans(&m, C4), [(start, bar(n) + 2 * STEP, 100)]);
        assert_eq!(row(&m, 1, C4)[last], HEAD);
        assert_eq!(&row(&m, 2, C4)[..3], [CONT, CONT, Cell::Empty]);
        shrink(&mut m, 2, 1);
        shrink(&mut m, 2, 0);
        assert_eq!(spans(&m, C4), [(start, bar(n), 100)]);
    }
}

#[test]
fn step_notes_extending_stops_at_the_end_of_the_range() {
    for n in [4, 3] {
        let mut m = song(n);
        let last = usize::from(n) * 4 - 1;
        toggle(&mut m, 1, C4, 1, last, 100).unwrap();
        grow(&mut m, 1, last, bar(n));
        assert_eq!(spans(&m, C4), [(bar(n) - STEP, bar(n), 100)]);
        toggle(&mut m, 1, C4, 1, 0, 100).unwrap();
        grow(&mut m, 1, 0, 2 * STEP);
        grow(&mut m, 1, 0, 2 * STEP);
        assert_eq!(spans(&m, C4)[0], (0, 2 * STEP, 100));
    }
}

#[test]
fn step_notes_shrinking_keeps_at_least_one_step() {
    let mut m = song(4);
    toggle(&mut m, 1, C4, 1, 5, 100).unwrap();
    shrink(&mut m, 1, 5);
    assert_eq!(spans(&m, C4), [(5 * STEP, 6 * STEP, 100)]);
    // Nothing to resize on an empty step.
    let before = m.clone();
    shrink(&mut m, 1, 9);
    grow(&mut m, 1, 9, u64::MAX);
    assert_eq!(
        crate::smf_io::encode(&m).unwrap(),
        crate::smf_io::encode(&before).unwrap()
    );
}

#[test]
fn step_notes_off_grid_notes_show_marked_and_snap_only_their_end() {
    let mut m = song(4);
    for (tick, status, velocity) in [(130, 0x93, 90), (300, 0x83, 0)] {
        edit::insert(&mut m.tracks[1], event(tick, status, C4, velocity));
    }
    let off = [
        Cell::Empty,
        Cell::Head { aligned: false },
        Cell::Continuation { aligned: false },
        Cell::Empty,
    ];
    assert_eq!(&row(&m, 1, C4)[..4], off);
    grow(&mut m, 1, 2, u64::MAX);
    assert_eq!(spans(&m, C4), [(130, 3 * STEP, 90)]);
    assert!(m.tracks[1]
        .events
        .iter()
        .filter(|e| e.data.first() == Some(&C4))
        .all(|e| e.status & 15 == 3));
    shrink(&mut m, 1, 2);
    assert_eq!(spans(&m, C4), [(130, 2 * STEP, 90)]);
    // The start stays off the grid, so the note keeps the off-grid mark.
    assert_eq!(&row(&m, 1, C4)[..3], [Cell::Empty, off[1], Cell::Empty]);
    shrink(&mut m, 1, 1);
    assert_eq!(spans(&m, C4), [(130, 2 * STEP, 90)]);
    // An end inside a step snaps back to the boundary below.
    let mut late = song(4);
    for (tick, status, velocity) in [(0, 0x90, 80), (300, 0x80, 0)] {
        edit::insert(&mut late.tracks[1], event(tick, status, C4, velocity));
    }
    shrink(&mut late, 1, 0);
    assert_eq!(spans(&late, C4), [(0, 2 * STEP, 80)]);
    toggle(&mut m, 1, C4, 1, 1, 100).unwrap();
    assert_eq!(spans(&m, C4), []);
}

#[test]
fn step_notes_new_notes_use_the_tracks_note_channel() {
    let mut m = song(4);
    edit::insert(&mut m.tracks[1], event(1000, 0x95, 70, 100));
    edit::insert(&mut m.tracks[1], event(1100, 0x85, 70, 0));
    toggle(&mut m, 1, C4, 1, 0, 100).unwrap();
    let statuses: Vec<_> = m.tracks[1]
        .events
        .iter()
        .filter(|e| e.data.first() == Some(&C4))
        .map(|e| e.status)
        .collect();
    assert_eq!(statuses, [0x95, 0x85]);
}
