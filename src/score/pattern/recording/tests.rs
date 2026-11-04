use super::*;
use crate::edit;
use crate::score::pattern::{body, Catalog};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
/// Free range on the letter pads, whose `Letter(i)` is C4..B4.
fn free(division: u16, tempo: u32) -> Recorder {
    let mut s = Settings::new(Key::from_signature(None), tempo);
    s.mode = Mode::Letters;
    // CC1 has its own test; these compare notes only.
    s.modulation = Modulation::Off;
    Recorder::new(division, s).unwrap()
}
const C: Pad = Pad::Letter(0);
const E: Pad = Pad::Letter(2);
const B: Pad = Pad::Letter(6);

#[test]
fn rec_origin_polyphony_fixed_tempo_and_round_trip() {
    let mut take = free(480, 500_000);
    take.start(ms(9000));
    take.press(C, ms(9000)).unwrap();
    assert!(take.press(C, ms(9100)).unwrap().is_empty());
    take.press(E, ms(9250)).unwrap();
    take.release(C, ms(9500)).unwrap();
    take.release(E, ms(11250)).unwrap();
    take.release_all(ms(30000)).unwrap(); // Esc silence is not pattern length.
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let id = take.commit(&mut m, None, Duration::ZERO).unwrap()[0];
    let c = Catalog::read(&m).unwrap();
    let p = &c.patterns[&id];
    assert_eq!(p.meters.len(), 2);
    let body = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        body.iter()
            .map(|e| (e.tick - p.tick, e.status, e.data[0]))
            .collect::<Vec<_>>(),
        vec![
            (0, 0x90, 60),
            (240, 0x90, 64),
            (480, 0x80, 60),
            (2160, 0x80, 64)
        ]
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("recording.mid");
    std::fs::write(&path, crate::smf_io::encode(&m).unwrap()).unwrap();
    let reopened = crate::smf_io::load(&path).unwrap();
    assert_eq!(
        crate::smf_io::encode(&m).unwrap(),
        crate::smf_io::encode(&reopened).unwrap()
    );
    assert_eq!(
        Catalog::read(&reopened).unwrap().patterns[&id].meters.len(),
        2
    );
}

#[test]
fn short_retrigger_is_ordered_and_never_overlaps_itself() {
    let mut take = free(480, 500_000);
    take.start(ms(0));
    take.press(C, ms(0)).unwrap();
    take.release(C, ms(0)).unwrap();
    take.press(C, ms(0)).unwrap();
    take.release_all(ms(0)).unwrap();
    let mut m = edit::tracks::new_song();
    let id = take.commit(&mut m, None, Duration::ZERO).unwrap()[0];
    let p = Catalog::read(&m).unwrap().patterns[&id].clone();
    let b = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        b.iter()
            .map(|e| (e.tick - p.tick, e.status))
            .collect::<Vec<_>>(),
        vec![(0, 0x90), (1, 0x80), (1, 0x90), (2, 0x80)]
    );
}

#[test]
fn existing_container_and_pattern_bytes_survive_long_recording() {
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let t = operations::create_source(&mut m).unwrap();
    let existing = operations::create(&mut m, t, None).unwrap();
    let original = m.tracks[t].events.clone();
    let mut take = free(480, 500_000);
    take.start(ms(0));
    take.press(B, ms(0)).unwrap();
    take.release_all(ms(4000)).unwrap();
    let id = take.commit(&mut m, Some(t), Duration::ZERO).unwrap()[0];
    let c = Catalog::read(&m).unwrap();
    assert_eq!(c.sources.len(), 1);
    assert_eq!(c.patterns[&existing].meters.len(), 1);
    assert_eq!(c.patterns[&id].meters.len(), 2); // Exact end boundary has no extra bar.
    for event in original {
        assert!(m.tracks[t]
            .events
            .iter()
            .any(|e| e.tick == event.tick && e.status == event.status && e.data == event.data));
    }
}

#[test]
fn empty_unfinished_and_smpte_takes_are_rejected() {
    let s = Settings::new(Key::from_signature(None), 500_000);
    assert!(Recorder::new(0, s).is_err());
    assert!(Recorder::new(0xe728, s).is_err());
    assert!(Recorder::new(480, Settings::new(Key::from_signature(None), 0)).is_err());
    let mut m = edit::tracks::new_song();
    let mut take = free(m.division, 500_000);
    assert!(!take.recorded());
    assert!(take.commit(&mut m, None, Duration::ZERO).is_err());
    take.start(ms(0));
    take.press(C, ms(0)).unwrap();
    assert!(take.recorded());
    assert!(take.commit(&mut m, None, Duration::ZERO).is_err());
    assert_eq!(Catalog::read(&m).unwrap().sources.len(), 0);
}

#[test]
fn tempo_sets_the_tick_scale() {
    let mut take = free(480, 1_000_000);
    take.start(ms(100));
    take.press(C, ms(100)).unwrap();
    take.release(C, ms(1100)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let id = take.commit(&mut m, None, Duration::ZERO).unwrap()[0];
    let p = Catalog::read(&m).unwrap().patterns[&id].clone();
    let b = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        b.last().map(|e| (e.tick - p.tick, e.status)),
        Some((480, 0x80))
    );
}

/// At 480 ticks and 500 ms per quarter, a 2000 ms bar.
fn loop_song() -> (crate::model::Music, usize) {
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let source = super::operations::create_source(&mut m).unwrap();
    (m, source)
}

#[test]
fn a_container_keeps_one_loop_that_each_phrase_overwrites() {
    let (mut m, source) = loop_song();
    let first = repeat(
        &mut m,
        Some(source),
        500_000,
        &[(ms(0), 60, 80), (ms(500), 60, 0)],
        &[],
        false,
        false,
    )
    .unwrap();
    let second = repeat(
        &mut m,
        Some(source),
        500_000,
        &[(ms(250), 64, 90), (ms(2000), 64, 0)],
        &[],
        false,
        false,
    )
    .unwrap();
    assert_eq!(first, second);
    let c = Catalog::read(&m).unwrap();
    let loops: Vec<_> = c
        .patterns
        .values()
        .filter(|p| p.kind == crate::score::pattern::Kind::Loop)
        .collect();
    assert_eq!(loops.len(), 1);
    assert_eq!((loops[0].name.as_str(), loops[0].meters.len()), ("Loop", 1));
    assert_eq!(
        looped(&m, first, 500_000).unwrap(),
        vec![(ms(250), 64, 90), (ms(2000), 64, 0)]
    );
}

#[test]
fn a_kept_loop_gives_up_only_the_pitches_the_phrase_starts() {
    let (mut m, source) = loop_song();
    let kick_and_hat = [
        (ms(0), 36, 100),
        (ms(100), 36, 0),
        (ms(1000), 42, 70),
        (ms(1100), 42, 0),
    ];
    let id = repeat(
        &mut m,
        Some(source),
        500_000,
        &kick_and_hat,
        &[],
        false,
        false,
    )
    .unwrap();
    let snare_and_kick = [
        (ms(500), 38, 90),
        (ms(600), 38, 0),
        (ms(1500), 36, 80),
        (ms(1600), 36, 0),
    ];
    repeat(
        &mut m,
        Some(source),
        500_000,
        &snare_and_kick,
        &[],
        true,
        false,
    )
    .unwrap();
    let mut got = looped(&m, id, 500_000).unwrap();
    got.sort();
    assert_eq!(
        got,
        vec![
            (ms(500), 38, 90),
            (ms(600), 38, 0),
            (ms(1000), 42, 70),
            (ms(1100), 42, 0),
            (ms(1500), 36, 80),
            (ms(1600), 36, 0),
        ]
    );
}

#[test]
fn a_loop_plays_its_first_bar_and_stops_what_still_sounds_at_its_end() {
    let (mut m, source) = loop_song();
    let id = repeat(
        &mut m,
        Some(source),
        500_000,
        &[(ms(0), 60, 80), (ms(500), 60, 0)],
        &[],
        false,
        false,
    )
    .unwrap();
    // As an edit in the event list would: a note past the bar, and one held over its end.
    let p = Catalog::read(&m).unwrap().patterns[&id].clone();
    super::operations::refill(
        &mut m,
        &p,
        vec![p.meters[0].clone(), p.meters[0].clone()],
        vec![
            super::note(0, 60, 80),
            super::note(240, 60, 0),
            super::note(1440, 62, 70),
            super::note(2400, 62, 0),
            super::note(2880, 67, 80),
            super::note(3000, 67, 0),
        ],
    )
    .unwrap();
    assert_eq!(
        looped(&m, id, 500_000).unwrap(),
        vec![
            (ms(0), 60, 80),
            (ms(250), 60, 0),
            (ms(1500), 62, 70),
            (ms(2000), 62, 0)
        ]
    );
}

#[test]
fn a_copy_of_the_loop_is_an_ordinary_pattern() {
    let (mut m, source) = loop_song();
    let id = repeat(
        &mut m,
        Some(source),
        500_000,
        &[(ms(0), 60, 80), (ms(500), 60, 0)],
        &[],
        false,
        false,
    )
    .unwrap();
    let copy = super::operations::create(&mut m, source, Some(id)).unwrap();
    let c = Catalog::read(&m).unwrap();
    assert_eq!(c.patterns[&copy].kind, crate::score::pattern::Kind::Normal);
    assert_eq!(
        looped(&m, copy, 500_000).unwrap(),
        looped(&m, id, 500_000).unwrap()
    );
}

#[test]
fn the_loop_is_not_placed_in_the_song_or_a_scene() {
    use crate::score::scene::arrangement::{self, PatternOrScene};
    let (mut m, source) = loop_song();
    let id = repeat(
        &mut m,
        Some(source),
        500_000,
        &[(ms(0), 60, 80), (ms(500), 60, 0)],
        &[],
        false,
        false,
    )
    .unwrap();
    let lane = arrangement::add_lane(&mut m, None).unwrap();
    let before = m.clone();
    assert_eq!(
        arrangement::place(&mut m, None, lane, 1, Some(PatternOrScene::Pattern(id))),
        Err(crate::message::LOOP_IS_NOT_PLACED.into())
    );
    assert_eq!(
        crate::smf_io::encode(&m).unwrap(),
        crate::smf_io::encode(&before).unwrap()
    );
    let scene = arrangement::create_scene(&mut m, "A", 1, 1).unwrap();
    let cell = crate::score::scene::read(&m).unwrap().scenes[0].lanes[0];
    let normal = super::operations::create(&mut m, source, None).unwrap();
    arrangement::place(
        &mut m,
        Some(scene),
        cell,
        1,
        Some(PatternOrScene::Pattern(normal)),
    )
    .unwrap();
    assert_eq!(
        arrangement::place(
            &mut m,
            Some(scene),
            cell,
            1,
            Some(PatternOrScene::Pattern(id))
        ),
        Err(crate::message::LOOP_IS_NOT_PLACED.into())
    );
}

#[test]
fn a_quantized_loop_plays_its_notes_on_sixteenths() {
    let (mut m, source) = loop_song();
    // At 120 BPM a sixteenth is 125 ms; the last key is a sixteenth or less before the bar line.
    let id = repeat(
        &mut m,
        Some(source),
        500_000,
        &[
            (ms(140), 60, 80),
            (ms(300), 60, 0),
            (ms(1960), 64, 90),
            (ms(2000), 64, 0),
        ],
        &[],
        false,
        true,
    )
    .unwrap();
    assert_eq!(
        looped(&m, id, 500_000).unwrap(),
        vec![
            (ms(0), 64, 90),
            (ms(125), 64, 0),
            (ms(125), 60, 80),
            (ms(250), 60, 0),
        ]
    );
}
