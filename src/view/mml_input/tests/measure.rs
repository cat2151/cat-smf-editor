use super::*;
use crate::play::transport;
use crate::view::palette::tests::{listed, open as open_palette, press, query};
use cat_smf_editor::model::Event;

const LABEL: &str = "Event list: MML / コード進行: 1小節を上書き";
/// Measure 2 in 4/4 at 480 ticks per quarter note.
const MEASURE: (u64, u64) = (1920, 3840);

/// A note from measure 1 sounds into measure 2; measure 2 has four quarter
/// notes; measure 3 has one.
fn song() -> App {
    let mut events = vec![];
    for (tick, pitch, length) in [
        (960, 55, 1440),
        (1920, 60, 480),
        (2400, 62, 480),
        (2880, 64, 480),
        (3360, 65, 480),
        (3840, 57, 480),
    ] {
        for (at, status, velocity) in [(tick, 0x90, 100), (tick + length, 0x80, 0)] {
            events.push(Event {
                tick: at,
                delta: 0,
                status,
                meta_type: 0,
                data: vec![pitch, velocity],
            });
        }
    }
    events.sort_by_key(|e| e.tick);
    let mut track = Track { events };
    cat_smf_editor::edit::refresh_deltas(&mut track).unwrap();
    crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![track],
    })
}

/// (tick, key, length) of every note in the track.
fn notes(app: &App) -> Vec<(u64, u8, u64)> {
    let loaded = app.loaded.as_ref().unwrap();
    let events = &loaded.music.tracks[0].events;
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| cat_smf_editor::score::note_view::note_on(e))
        .map(|(i, e)| {
            (
                e.tick,
                e.data[0],
                loaded.note_views[0].durations[i].unwrap(),
            )
        })
        .collect()
}

fn assert_looping(app: &App) {
    assert_eq!(app.playback.repeat, Some(MEASURE));
    assert!(transport::timeline::running(app));
}

#[test]
fn the_palette_repeats_the_cursor_measure_and_typed_mml_overwrites_its_notes() {
    let mut app = song();
    let ctx = egui::Context::default();
    app.track_view.cursor_tick = Some(2400);
    frame(&mut app, &ctx, vec![]);
    let original = notes(&app);
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();

    for q in ["上書き", "1小節", "overwrite", "measure"] {
        open_palette(&mut app, &ctx);
        query(&mut app, &ctx, q);
        let found = listed(&mut app);
        eprintln!("query {q:?}: {found:?}");
        assert_eq!(found.first().map(String::as_str), Some(LABEL));
        if q == "measure" {
            press(&mut app, &ctx, egui::Key::Enter);
        } else {
            press(&mut app, &ctx, egui::Key::Escape);
        }
    }
    assert!(!app.palette.is_open());
    frame(&mut app, &ctx, vec![]);
    assert!(app.mml_input.open);
    assert!(app.mml_input.repeat_session);
    assert_eq!(app.mml_input.tick, MEASURE.0);
    assert_eq!(app.track_view.cursor_tick, Some(MEASURE.0));
    assert_looping(&app);
    assert_eq!(notes(&app), original);

    // G overwrites C; the rest removes D; the note from measure 1 now ends
    // where the input starts.
    frame(&mut app, &ctx, vec![egui::Event::Text("o5 l4 g r".into())]);
    assert_eq!(
        notes(&app),
        [
            (960, 55, 960),
            (1920, 67, 480),
            (2880, 64, 480),
            (3360, 65, 480),
            (3840, 57, 480),
        ]
    );
    assert_looping(&app);
    frame(&mut app, &ctx, vec![egui::Event::Text(" b".into())]);
    assert_eq!(
        notes(&app),
        [
            (960, 55, 960),
            (1920, 67, 480),
            (2880, 71, 480),
            (3360, 65, 480),
            (3840, 57, 480),
        ]
    );
    assert_looping(&app);

    frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
    frame(&mut app, &ctx, vec![]);
    assert!(!app.mml_input.open);
    assert_eq!(notes(&app)[2], (2880, 71, 480));
    assert_looping(&app);
    app.undo_music().unwrap();
    assert!(!app.undo.available());
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}

#[test]
fn cancel_restores_the_measure_and_keeps_looping() {
    let mut app = song();
    let ctx = egui::Context::default();
    app.track_view.cursor_tick = Some(3000);
    frame(&mut app, &ctx, vec![]);
    let before = smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    open_palette(&mut app, &ctx);
    query(&mut app, &ctx, "measure");
    press(&mut app, &ctx, egui::Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.mml_input.open);
    frame(&mut app, &ctx, vec![egui::Event::Text("r1".into())]);
    assert_eq!(notes(&app), [(960, 55, 960), (3840, 57, 480)]);
    frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
    assert!(!app.mml_input.open);
    assert_eq!(
        smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!app.undo.available());
    assert_looping(&app);
}

#[test]
fn a_new_song_repeats_its_first_measure() {
    let mut app = song();
    let ctx = egui::Context::default();
    app.new_smf_unchecked();
    frame(&mut app, &ctx, vec![]);
    open_palette(&mut app, &ctx);
    query(&mut app, &ctx, "measure");
    press(&mut app, &ctx, egui::Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.mml_input.open && app.mml_input.repeat_session);
    assert_eq!(app.playback.repeat, Some((0, 1920)));
    frame(&mut app, &ctx, vec![egui::Event::Text("o5 l2 d".into())]);
    let music = &app.loaded.as_ref().unwrap().music;
    let keys: Vec<_> = music.tracks[app.track]
        .events
        .iter()
        .filter(|e| cat_smf_editor::score::note_view::note_on(e))
        .map(|e| (e.tick, e.data[0]))
        .collect();
    assert_eq!(keys, [(0, 62)]);
}
