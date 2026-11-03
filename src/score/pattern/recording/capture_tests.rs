use super::*;
use crate::{
    edit,
    score::pattern::{body, record, Catalog, Record},
};

const GAP: u64 = 2000;

/// Keys as `(at, pitch, on)` on container 7, then everything left.
fn phrases(keys: &[(u64, u8, bool)], other: &[(u64, u8, bool)]) -> Vec<Phrase> {
    let mut k = Keys::default();
    let mut taken = vec![];
    for &(at, pitch, on) in keys {
        k.key(at, pitch, if on { 80 } else { 0 }, Some(7), GAP);
        taken.extend(k.take(at, GAP));
    }
    for &(at, pitch, on) in other {
        k.key(at, pitch, if on { 80 } else { 0 }, None, GAP);
    }
    taken.extend(k.finish());
    taken
}
fn n(start: u64, length: u64, pitch: u8) -> Note {
    Note {
        start,
        length,
        pitch,
        velocity: 80,
    }
}

#[test]
fn silence_of_the_gap_or_another_container_starts_a_phrase() {
    let p = phrases(
        &[
            (1000, 60, true),
            (1200, 64, true),
            (1500, 60, false),
            (2900, 64, false),
            // Sounding notes hold the phrase open however long the key is held.
            (4800, 62, true),
            (9000, 62, false),
            // Two seconds of silence.
            (11_000, 65, true),
            (11_100, 65, false),
        ],
        &[(11_200, 67, true), (11_300, 67, false)],
    );
    assert_eq!(
        p.iter()
            .map(|p| (p.start, p.container, p.notes.clone()))
            .collect::<Vec<_>>(),
        vec![
            (
                1000,
                Some(7),
                vec![n(0, 500, 60), n(200, 1700, 64), n(3800, 4200, 62)]
            ),
            (11_000, Some(7), vec![n(0, 100, 65)]),
            (11_200, None, vec![n(0, 100, 67)]),
        ]
    );
}

#[test]
fn silence_ends_the_phrase_without_another_key() {
    let mut k = Keys::default();
    k.key(1000, 60, 80, Some(7), GAP);
    k.key(1100, 60, 0, Some(7), GAP);
    assert!(k.take(3099, GAP).is_empty());
    assert_eq!(k.take(3100, GAP).len(), 1);
    assert!(k.finish().is_empty());
}

#[test]
fn keys_left_sounding_end_at_the_last_key() {
    let p = phrases(
        &[
            (500, 60, false),
            (1000, 60, true),
            (1300, 60, true),
            (1400, 62, true),
        ],
        &[],
    );
    assert_eq!(
        p[0].notes,
        vec![n(0, 300, 60), n(300, 100, 60), n(400, 0, 62)]
    );
}

#[test]
fn each_marker_lists_its_bars_and_takes_them_as_a_normal_pattern() {
    let mut m = edit::tracks::new_song();
    m.division = 480;
    // 120 BPM: 500 ms per quarter. The first phrase ends after bar 1.
    let first = Phrase {
        start: 0,
        container: None,
        notes: vec![n(0, 500, 60), n(500, 1, 60), n(1750, 500, 64)],
    };
    let id = append(&mut m, None, 500_000, &first, &[], "2026-11-01 10:00:00").unwrap();
    let track = Catalog::read(&m).unwrap().patterns[&id].track;
    let second = Phrase {
        start: 9000,
        container: None,
        notes: vec![n(0, 250, 67)],
    };
    append(
        &mut m,
        Some(track),
        500_000,
        &second,
        &[],
        "2026-11-01 10:00:09",
    )
    .unwrap();
    let listed = captured(&m).unwrap();
    // Newest first; bars 0-1, a bar of rest, then bar 3.
    assert_eq!(
        listed
            .iter()
            .map(|c| (c.pattern, c.stamp.as_str(), c.range, c.meters.len()))
            .collect::<Vec<_>>(),
        vec![
            (id, "2026-11-01 10:00:09", (5760, 7680), 1),
            (id, "2026-11-01 10:00:00", (0, 3840), 2),
        ]
    );
    let tick = |start, length, pitch| TickNote {
        start,
        length,
        pitch,
    };
    assert_eq!(
        listed[1].notes,
        vec![tick(0, 480, 60), tick(480, 1, 60), tick(1680, 480, 64)]
    );
    assert_eq!(listed[1].end(), 2160);
    assert_eq!(listed[0].notes, vec![tick(0, 240, 67)]);

    let taken = take(&mut m, &listed[1]).unwrap();
    let c = Catalog::read(&m).unwrap();
    let p = &c.patterns[&taken];
    assert_eq!((p.kind, p.track, p.meters.len()), (Kind::Normal, track, 2));
    let body = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        body.iter()
            .map(|e| (e.tick - p.tick, e.status, e.data[0]))
            .collect::<Vec<_>>(),
        vec![
            (0, 0x90, 60),
            (480, 0x80, 60),
            (480, 0x90, 60),
            (481, 0x80, 60),
            (1680, 0x90, 64),
            (2160, 0x80, 64),
        ]
    );
    // The Capture pattern keeps its phrases.
    assert_eq!(captured(&m).unwrap(), listed);
}

/// Body events of `id` as `(bar, tick in bar, status, first data byte or marker text)`.
fn bars(m: &Music, id: u64) -> Vec<(u64, u64, u8, String)> {
    let c = Catalog::read(m).unwrap();
    let p = &c.patterns[&id];
    let bar = u64::from(m.division) * 4;
    body::extract(&m.tracks[p.track].events, p.range().unwrap())
        .unwrap()
        .iter()
        .map(|e| {
            let at = e.tick - p.tick;
            let data = if e.status == 0xff {
                String::from_utf8_lossy(&e.data).into_owned()
            } else {
                e.data[0].to_string()
            };
            let status = if e.status == 0xff {
                e.meta_type
            } else {
                e.status
            };
            (at / bar, at % bar, status, data)
        })
        .collect()
}

#[test]
fn phrases_append_to_one_capture_pattern_from_bar_one_with_a_bar_between() {
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let first = Phrase {
        start: 0,
        container: None,
        // Ends inside bar 2 at 120 BPM.
        notes: vec![n(0, 2500, 60)],
    };
    let id = append(&mut m, None, 500_000, &first, &[], "2026-11-01 10:00:00").unwrap();
    let c = Catalog::read(&m).unwrap();
    let p = &c.patterns[&id];
    assert_eq!((p.kind, p.name.as_str()), (Kind::Capture, "Capture"));
    let track = p.track;
    // A renamed Capture pattern still collects the keys.
    let meta = m.tracks[track]
        .events
        .iter_mut()
        .find(|e| matches!(record(e), Ok(Some(Record::Pattern(_)))))
        .unwrap();
    let Ok(Some(Record::Pattern(mut r))) = record(meta) else {
        unreachable!()
    };
    r.name = "Ideas".into();
    *meta = crate::score::pattern::event(meta.tick, &Record::Pattern(r));
    let second = Phrase {
        start: 9000,
        container: Some(p.container),
        notes: vec![n(0, 250, 64)],
    };
    assert_eq!(
        append(
            &mut m,
            Some(track),
            500_000,
            &second,
            &[],
            "2026-11-01 10:00:09"
        )
        .unwrap(),
        id
    );
    let c = Catalog::read(&m).unwrap();
    assert_eq!(c.patterns.len(), 1);
    assert_eq!(c.patterns[&id].name, "Ideas");
    assert_eq!(c.patterns[&id].meters.len(), 4);
    assert_eq!(
        bars(&m, id),
        vec![
            (0, 0, 6, "2026-11-01 10:00:00".into()),
            (0, 0, 0x90, "60".into()),
            (1, 480, 0x80, "60".into()),
            (3, 0, 6, "2026-11-01 10:00:09".into()),
            (3, 0, 0x90, "64".into()),
            (3, 240, 0x80, "64".into()),
        ]
    );
}

#[test]
fn a_capture_grows_past_the_patterns_after_it_and_stays_out_of_the_song_export() {
    use crate::score::scene::{arrangement as grid, arrangement::PatternOrScene};
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let t = operations::create_source(&mut m).unwrap();
    let phrase = |pitch| Phrase {
        start: 0,
        container: None,
        notes: vec![n(0, 250, pitch)],
    };
    let capture = append(&mut m, Some(t), 500_000, &phrase(60), &[], "a").unwrap();
    let first = captured(&m).unwrap().remove(0);
    let placed = take(&mut m, &first).unwrap();
    append(&mut m, Some(t), 500_000, &phrase(62), &[], "b").unwrap();
    let c = Catalog::read(&m).unwrap();
    assert_eq!(c.patterns[&capture].meters.len(), 3);
    assert_eq!(
        bars(&m, capture)
            .iter()
            .filter(|e| e.2 == 0x90)
            .map(|e| (e.0, e.3.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "60"), (2, "62")]
    );
    let lane = grid::add_lane(&mut m, None).unwrap();
    grid::place(&mut m, None, lane, 1, Some(PatternOrScene::Pattern(placed))).unwrap();
    let song = crate::score::performance::song(&m).unwrap();
    let events: Vec<_> = song.tracks.iter().flat_map(|t| &t.events).collect();
    assert!(events
        .iter()
        .any(|e| e.status & 0xf0 == 0x90 && e.data[0] == 60));
    assert!(!events
        .iter()
        .any(|e| e.status & 0xf0 == 0x90 && e.data[0] != 60));
    assert!(!events.iter().any(|e| e.status == 0xff && e.meta_type == 6));
}

#[test]
fn a_phrase_keeps_each_key_velocity_and_its_cc1_before_the_notes() {
    let mut k = Keys::default();
    k.key(1000, 60, 93, None, GAP);
    k.key(1500, 60, 0, None, GAP);
    k.key(1500, 62, 127, None, GAP);
    k.key(2000, 62, 0, None, GAP);
    let phrase = k.finish().remove(0);
    assert_eq!(phrase.notes[0].velocity, 93);
    assert_eq!(phrase.end(), 1000);
    let mut m = edit::tracks::new_song();
    m.division = 480;
    // At 120 BPM 1 ms is 0.96 ticks; 1000 ms is the phrase's end, so not written.
    let id = append(
        &mut m,
        None,
        500_000,
        &phrase,
        &[(0, 5), (500, 9), (1000, 12)],
        "a",
    )
    .unwrap();
    let c = Catalog::read(&m).unwrap();
    let p = &c.patterns[&id];
    let body = body::extract(&m.tracks[p.track].events, p.range().unwrap()).unwrap();
    assert_eq!(
        body.iter()
            .filter(|e| e.status != 0xff)
            .map(|e| (e.tick - p.tick, e.status, e.data.clone()))
            .collect::<Vec<_>>(),
        vec![
            (0, 0xb0, vec![1, 5]),
            (0, 0x90, vec![60, 93]),
            (480, 0xb0, vec![1, 9]),
            (480, 0x80, vec![60, 0]),
            (480, 0x90, vec![62, 127]),
            (960, 0x80, vec![62, 0]),
        ]
    );
}

/// A song with a container holding one normal pattern.
fn song_with_a_pattern() -> (Music, usize, u64) {
    let mut m = edit::tracks::new_song();
    m.division = 480;
    let t = operations::create_source(&mut m).unwrap();
    let normal = operations::create(&mut m, t, None).unwrap();
    (m, t, normal)
}

fn played(pitch: u8) -> Phrase {
    Phrase {
        start: 0,
        container: None,
        notes: vec![n(0, 250, pitch), n(500, 250, pitch + 4)],
    }
}

#[test]
fn carry_copies_a_capture_past_the_normal_patterns_of_the_older_song() {
    let (mut into, t, normal) = song_with_a_pattern();
    let mut from = into.clone();
    let capture = append(&mut from, Some(t), 500_000, &played(60), &[], "a").unwrap();
    // The older song's normal pattern covers where the Capture is stored in `from`.
    let bar = Meter {
        numerator: 4,
        exponent: 2,
        ticks: 1920,
    };
    operations::resize(&mut into, normal, vec![bar; 4]).unwrap();
    let before = Catalog::read(&into).unwrap().patterns[&normal].clone();
    let body = bars(&into, normal);
    carry(&from, &mut into).unwrap();
    let c = Catalog::read(&into).unwrap();
    assert_eq!(c.patterns.len(), 2);
    assert_eq!(c.patterns[&capture].kind, Kind::Capture);
    assert_eq!(captured(&into).unwrap(), captured(&from).unwrap());
    let after = &c.patterns[&normal];
    assert_eq!(
        (after.track, after.tick, after.meters.len()),
        (before.track, before.tick, 4)
    );
    assert_eq!(bars(&into, normal), body);
}

#[test]
fn carry_replaces_the_capture_of_the_same_id() {
    let (mut into, t, _) = song_with_a_pattern();
    append(&mut into, Some(t), 500_000, &played(60), &[], "a").unwrap();
    let mut from = into.clone();
    append(&mut from, Some(t), 500_000, &played(62), &[], "b").unwrap();
    carry(&from, &mut into).unwrap();
    assert_eq!(Catalog::read(&into).unwrap().patterns.len(), 2);
    let listed = captured(&into).unwrap();
    assert_eq!(listed, captured(&from).unwrap());
    assert_eq!(
        listed.iter().map(|c| c.stamp.as_str()).collect::<Vec<_>>(),
        vec!["b", "a"]
    );
}

#[test]
fn carry_brings_a_container_only_the_newer_song_has() {
    let mut into = edit::tracks::new_song();
    into.division = 480;
    let mut from = into.clone();
    let capture = append(&mut from, None, 500_000, &played(60), &[], "a").unwrap();
    carry(&from, &mut into).unwrap();
    let (old, new) = (Catalog::read(&from).unwrap(), Catalog::read(&into).unwrap());
    assert_eq!(
        new.tracks.keys().collect::<Vec<_>>(),
        old.tracks.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        new.patterns[&capture].container,
        old.patterns[&capture].container
    );
    assert_eq!(captured(&into).unwrap(), captured(&from).unwrap());
    let name = |m: &Music, track: usize| {
        m.tracks[track]
            .events
            .iter()
            .find(|e| e.status == 0xff && e.meta_type == 3)
            .map(|e| e.data.clone())
    };
    let track = |c: &Catalog| c.patterns[&capture].track;
    assert_eq!(name(&into, track(&new)), name(&from, track(&old)));
}

#[test]
fn carry_without_a_capture_leaves_the_song_as_it_was() {
    let (from, _, _) = song_with_a_pattern();
    let (mut into, t, _) = song_with_a_pattern();
    operations::create(&mut into, t, None).unwrap();
    let bytes = crate::smf_io::encode(&into).unwrap();
    carry(&from, &mut into).unwrap();
    assert_eq!(crate::smf_io::encode(&into).unwrap(), bytes);
}
