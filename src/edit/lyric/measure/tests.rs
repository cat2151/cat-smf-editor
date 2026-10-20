use super::{moras, replace};
use crate::{
    edit::lyric::{set, text_at, LYRIC},
    model::{Event, Music, Track},
    score::note_view::{note_on, NoteView},
};

const MEASURE: (u64, u64) = (1920, 3840);

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// Quarter notes of `(tick, pitch, lyric)` on track 1, and lyrics of `orphans` at ticks without a note.
fn song(notes: &[(u64, u8, &str)], orphans: &[(u64, &str)]) -> Music {
    let mut events = Vec::new();
    for &(tick, pitch, _) in notes {
        events.push(event(tick, 0x90, 0, &[pitch, 100]));
        events.push(event(tick + 480, 0x80, 0, &[pitch, 0]));
    }
    for &(tick, text) in orphans {
        events.push(event(tick, 0xff, LYRIC, text.as_bytes()));
    }
    events.sort_by_key(|e| e.tick);
    let end = events.last().map_or(0, |e| e.tick);
    events.push(event(end, 0xff, 0x2f, &[]));
    let mut track = Track { events };
    crate::edit::refresh_deltas(&mut track).unwrap();
    for &(tick, _, text) in notes {
        let on = track
            .events
            .iter()
            .position(|e| e.tick == tick && note_on(e))
            .unwrap();
        set(&mut track, on, text).unwrap();
    }
    Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }, track],
    }
}

/// `(tick, lyric)` of every lyric event of track 1.
fn lyrics(music: &Music) -> Vec<(u64, String)> {
    music.tracks[1]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == LYRIC)
        .map(|e| (e.tick, String::from_utf8(e.data.clone()).unwrap()))
        .collect()
}

fn owned(list: &[(u64, &str)]) -> Vec<(u64, String)> {
    list.iter().map(|&(t, s)| (t, s.to_string())).collect()
}

/// Measure 2 holds four note ticks (a chord at 2880) and an orphan lyric; measures 1 and 3 hold
/// a sung note each and an orphan at 3900.
fn measure_song() -> Music {
    song(
        &[
            (1440, 59, "あ"),
            (1920, 60, ""),
            (2400, 62, "ぬ"),
            (2880, 64, ""),
            (2880, 67, ""),
            (3360, 65, "け"),
            (3840, 57, "お"),
        ],
        &[(2000, "x"), (3900, "y")],
    )
}

#[test]
fn each_character_overwrites_one_note_in_the_measure_and_orphans_in_it_go() {
    let mut music = measure_song();
    replace(&mut music, 1, MEASURE, "どんぐり").unwrap();
    assert_eq!(
        lyrics(&music),
        owned(&[
            (1440, "あ"),
            (1920, "ど"),
            (2400, "ん"),
            (2880, "ぐ"),
            (3360, "り"),
            (3840, "お"),
            (3900, "y"),
        ])
    );
    // The lyric sits just before its note, as `set` puts it.
    let events = &music.tracks[1].events;
    let at = events
        .iter()
        .position(|e| e.tick == 2880 && e.meta_type == LYRIC)
        .unwrap();
    assert!(note_on(&events[at + 1]));
    let reread = crate::ffi::parse(&crate::smf_io::encode(&music).unwrap()).unwrap();
    assert_eq!(lyrics(&reread), lyrics(&music));
}

#[test]
fn fewer_moras_keep_the_later_lyrics_and_moras_past_a_full_measure_go() {
    let mut music = measure_song();
    replace(&mut music, 1, MEASURE, "きゃ り").unwrap();
    assert_eq!(
        text_at(&music.tracks[1].events, 1920).as_deref(),
        Some("きゃ")
    );
    assert_eq!(
        text_at(&music.tracks[1].events, 2400).as_deref(),
        Some("り")
    );
    assert_eq!(text_at(&music.tracks[1].events, 2880), None);
    assert_eq!(
        text_at(&music.tracks[1].events, 3360).as_deref(),
        Some("け")
    );
    assert!(!lyrics(&music).contains(&(2000, "x".into())));

    let mut music = measure_song();
    replace(&mut music, 1, MEASURE, "どんぐりころころ").unwrap();
    let got = lyrics(&music);
    assert_eq!(got.len(), 7);
    assert!(got.contains(&(3840, "お".into())));

    // No moras still removes the orphans; the notes keep their lyrics.
    let mut music = measure_song();
    replace(&mut music, 1, MEASURE, "").unwrap();
    assert_eq!(
        lyrics(&music),
        owned(&[
            (1440, "あ"),
            (2400, "ぬ"),
            (3360, "け"),
            (3840, "お"),
            (3900, "y")
        ])
    );
}

fn texts(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// `(tick, pitch, duration, channel, velocity, lyric)` of each Note On of track 1.
fn notes(music: &Music) -> Vec<(u64, u8, u64, u8, u8, Option<String>)> {
    let events = &music.tracks[1].events;
    let view = NoteView::new(events);
    (0..events.len())
        .filter(|&i| note_on(&events[i]))
        .map(|i| {
            let e = &events[i];
            (
                e.tick,
                e.data[0],
                view.durations[i].unwrap(),
                e.status & 15,
                e.data[1],
                text_at(events, e.tick),
            )
        })
        .collect()
}

#[test]
fn small_kana_join_the_previous_character_and_whitespace_only_separates() {
    assert_eq!(moras("どんぐりころころ").len(), 8);
    assert_eq!(moras("きゃりー"), texts(&["きゃ", "り", "ー"]));
    assert_eq!(moras("キャ　リ ッ"), texts(&["キャ", "リ", "ッ"]));
    assert_eq!(moras(" ゃど"), texts(&["ゃ", "ど"]));
    assert!(moras("  ").is_empty());
}

#[test]
fn moras_past_the_notes_get_notes_on_60_like_the_note_before() {
    let mut music = song(&[(1920, 64, "")], &[]);
    // The note's channel 3 and velocity 90 carry over to the added notes.
    for e in &mut music.tracks[1].events {
        if e.status & 0xf0 == 0x90 || e.status & 0xf0 == 0x80 {
            e.status |= 3;
        }
        if note_on(e) {
            e.data[1] = 90;
        }
    }
    replace(&mut music, 1, MEASURE, "どんぐり").unwrap();
    let some = |s: &str| Some(s.to_string());
    assert_eq!(
        notes(&music),
        vec![
            (1920, 64, 480, 3, 90, some("ど")),
            (2400, 60, 480, 3, 90, some("ん")),
            (2880, 60, 480, 3, 90, some("ぐ")),
            (3360, 60, 480, 3, 90, some("り")),
        ]
    );
    let reread = crate::ffi::parse(&crate::smf_io::encode(&music).unwrap()).unwrap();
    assert_eq!(notes(&reread), notes(&music));
}

#[test]
fn a_measure_without_notes_gets_eighth_notes_on_60_from_its_start_until_its_end() {
    let mut music = song(&[(1440, 59, "あ"), (3840, 57, "お")], &[]);
    replace(&mut music, 1, MEASURE, "どんぐりころころどんぐりこ").unwrap();
    let got = notes(&music);
    let added: Vec<_> = got
        .iter()
        .filter(|n| (MEASURE.0..MEASURE.1).contains(&n.0))
        .cloned()
        .collect();
    let expected: Vec<_> = moras("どんぐりころころ")
        .into_iter()
        .enumerate()
        .map(|(i, mora)| (1920 + 240 * i as u64, 60, 240, 0, 100, Some(mora)))
        .collect();
    assert_eq!(added, expected);
    // The next measure keeps its note and lyric.
    assert_eq!(
        got.last().unwrap(),
        &(3840, 57, 480, 0, 100, Some("お".into()))
    );
    assert_eq!(got.len(), 10);
}

#[test]
fn a_lone_note_at_the_measure_start_shortens_until_every_mora_fits() {
    let mut music = song(&[(1920, 64, "")], &[]);
    replace(&mut music, 1, MEASURE, "どんぐりころころ").unwrap();
    let expected: Vec<_> = moras("どんぐりころころ")
        .into_iter()
        .enumerate()
        .map(|(i, mora)| {
            let pitch = if i == 0 { 64 } else { 60 };
            (1920 + 240 * i as u64, pitch, 240, 0, 100, Some(mora))
        })
        .collect();
    assert_eq!(notes(&music), expected);
}
