//! `performance` borrows what was built with the song, and never draws a stale one.
use super::{performance, sounding, Mode, EXPANSIONS_BUILT};
use crate::app::App;
use crate::view::patterns::{library, mode};
use crate::view::tests::unplaced::{fixture, CONTAINER};
use cat_smf_editor::{
    edit,
    model::{Event, Track},
    score::same_measure,
};
use eframe::egui;

/// Track 2 plays measure 1 again in measures 2 and 3 by reference.
const REFERRING: usize = 2;

fn note(tick: u64, status: u8, pitch: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, if status == 0x90 { 100 } else { 0 }],
    }
}

/// The pattern fixture plus a track with same-measure references. Returns the app and p1.
fn song() -> (App, u64) {
    let (a, p1) = fixture();
    let mut music = a.loaded.unwrap().music;
    let mut track = Track { events: vec![] };
    for event in [
        note(0, 0x90, 60),
        note(480, 0x80, 60),
        same_measure::event(1920, 1),
        same_measure::event(3840, 1),
    ] {
        edit::insert(&mut track, event);
    }
    edit::refresh_deltas(&mut track).unwrap();
    music.tracks.push(track);
    assert_eq!(music.tracks.len() - 1, REFERRING);
    (crate::view::tests::app(music), p1)
}

/// `(tick, status, meta type, data)` per event, durations and end.
type Plain = (Vec<(u64, u8, u8, Vec<u8>)>, Vec<Option<u64>>, u64);

fn flat(s: super::Sounding<'_>) -> Plain {
    let events = s
        .events
        .iter()
        .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
        .collect();
    (events, s.durations.into_owned(), s.end)
}

fn plain(a: &App) -> Vec<Plain> {
    performance(a).into_iter().map(flat).collect()
}

/// What `performance` returned when it expanded every track on every call.
fn rebuilt(a: &App) -> Vec<Plain> {
    let tracks = a.loaded.as_ref().map_or(0, |l| l.music.tracks.len());
    (0..tracks)
        .map(|t| sounding(a, t, Mode::Position).unwrap_or_default())
        .map(flat)
        .collect()
}

fn pitches(a: &App, track: usize) -> Vec<u8> {
    performance(a)[track]
        .events
        .iter()
        .filter(|e| e.status & 0xf0 == 0x90)
        .map(|e| e.data[0])
        .collect()
}

#[test]
fn performance_matches_rebuilding_after_edit_undo_pattern_edit_and_reload() {
    let (mut a, p1) = song();
    assert_eq!(plain(&a), rebuilt(&a));
    assert_eq!(pitches(&a, REFERRING), vec![60, 60, 60]);

    a.select_track(REFERRING);
    a.edit_music(|m| {
        for e in &mut m.tracks[REFERRING].events {
            if e.status & 0xe0 == 0x80 {
                e.data[0] = 64;
            }
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(pitches(&a, REFERRING), vec![64, 64, 64], "edit");
    assert_eq!(plain(&a), rebuilt(&a), "edit");

    a.undo_music().unwrap();
    assert_eq!(pitches(&a, REFERRING), vec![60, 60, 60], "undo");
    assert_eq!(plain(&a), rebuilt(&a), "undo");

    let before = plain(&a);
    library::edit(&mut a, p1).unwrap();
    assert!(a.patterns.editing.as_ref().unwrap().standalone);
    assert_eq!(pitches(&a, CONTAINER), vec![60], "pattern edit start");
    assert_eq!(plain(&a), rebuilt(&a), "pattern edit start");
    assert_ne!(plain(&a), before, "pattern edit start");

    mode::close_workspace(&mut a);
    assert!(a.patterns.editing.is_none());
    assert_eq!(plain(&a), rebuilt(&a), "pattern edit end");
    assert_eq!(plain(&a), before, "pattern edit end");

    let root = std::env::temp_dir().join(format!("cat-smf-performance-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("reload.mid");
    let mut changed = a.loaded.as_ref().unwrap().music.clone();
    for e in &mut changed.tracks[REFERRING].events {
        if e.status & 0xe0 == 0x80 {
            e.data[0] = 67;
        }
    }
    std::fs::write(&path, cat_smf_editor::smf_io::encode(&changed).unwrap()).unwrap();
    a.load_mid_unchecked(std::fs::canonicalize(&path).unwrap());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&root).unwrap();
    assert!(a.load_error.is_none(), "{:?}", a.load_error);
    assert_eq!(pitches(&a, REFERRING), vec![67, 67, 67], "reload");
    assert_eq!(plain(&a), rebuilt(&a), "reload");
}

#[test]
fn frames_without_song_changes_do_not_expand_again() {
    let (mut a, _) = song();
    EXPANSIONS_BUILT.with(|n| n.set(0));
    a.loaded.as_mut().unwrap().refresh();
    a.select_track(REFERRING);
    a.track_overview.open = true;
    a.piano_roll.open = true;
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1500.0, 900.0),
        )),
        ..Default::default()
    };
    for _ in 0..2 {
        let _ = ctx.run(input(), |ctx| crate::view::show(&mut a, ctx));
    }
    assert_eq!(EXPANSIONS_BUILT.with(|n| n.get()), 1);
}
