use super::*;
use crate::score::pattern::body;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
/// Free range on the letter pads, whose `Letter(i)` is C4..B4.
fn free(division: u16, tempo: u32) -> Recorder {
    let mut s = Settings::new(Key::from_signature(None), tempo);
    s.layout = Layout::Letters;
    s.start = Start::Key;
    Recorder::new(division, s).unwrap()
}
const C: Pad = Pad::Letter(0);
const E: Pad = Pad::Letter(2);
const B: Pad = Pad::Letter(6);

#[test]
fn first_key_origin_polyphony_fixed_tempo_and_round_trip() {
    let mut take = free(480, 500_000);
    take.press(C, ms(9000)).unwrap();
    assert!(take.press(C, ms(9100)).unwrap().is_empty());
    take.press(E, ms(9250)).unwrap();
    take.release(C, ms(9500)).unwrap();
    take.release(E, ms(11250)).unwrap();
    take.release_all(ms(30000)).unwrap(); // Esc silence is not pattern length.
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let id = take.commit(&mut m, None).unwrap()[0];
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
    take.press(C, ms(0)).unwrap();
    take.release(C, ms(0)).unwrap();
    take.press(C, ms(0)).unwrap();
    take.release_all(ms(0)).unwrap();
    let mut m = edit::tracks::new_song();
    let id = take.commit(&mut m, None).unwrap()[0];
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
    take.press(B, ms(0)).unwrap();
    take.release_all(ms(4000)).unwrap();
    let id = take.commit(&mut m, Some(t)).unwrap()[0];
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
    assert!(take.commit(&mut m, None).is_err());
    take.press(C, ms(0)).unwrap();
    assert!(take.recorded());
    assert!(take.commit(&mut m, None).is_err());
    assert_eq!(Catalog::read(&m).unwrap().sources.len(), 0);
}

#[test]
fn tempo_sets_the_tick_scale() {
    let mut take = free(480, 1_000_000);
    take.press(C, ms(100)).unwrap();
    take.release(C, ms(1100)).unwrap();
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let id = take.commit(&mut m, None).unwrap()[0];
    let p = Catalog::read(&m).unwrap().patterns[&id].clone();
    let b = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        b.last().map(|e| (e.tick - p.tick, e.status)),
        Some((480, 0x80))
    );
}
