use crate::app::App;
use crate::play::transport;
use crate::view::{
    edit_menu::lyric_tests::{settle, wav},
    palette::tests::{frame, listed, open, press, query},
    tests::app,
};
use cat_smf_editor::edit::lyric::{set, LYRIC};
use cat_smf_editor::model::{Event, Music, Track};
use cat_smf_editor::score::note_view::note_on;
use eframe::egui::{self, Key};
use std::sync::atomic::{AtomicUsize, Ordering};

const LABEL: &str = "Score: VOICEVOX: 1小節の歌詞を差し替え";
/// Measure 2 in 4/4 at 480 ticks per quarter note.
pub(super) const MEASURE: (u64, u64) = (1920, 3840);

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// The singer's track: measure 1 has a sung note, measure 2 four note ticks (a chord at 2880),
/// one sung, and a lyric at 2000 without a note; measure 3 has a sung note.
pub(super) fn song() -> Music {
    let notes = [
        (480, 59, "あ"),
        (1920, 60, ""),
        (2400, 62, "ぬ"),
        (2880, 64, ""),
        (2880, 67, ""),
        (3360, 65, ""),
        (3840, 57, "お"),
    ];
    let mut events = vec![
        event(0, 0xff, 0x03, "ずんだもん".as_bytes()),
        event(2000, 0xff, LYRIC, b"x"),
    ];
    for &(tick, pitch, _) in &notes {
        events.push(event(tick, 0x90, 0, &[pitch, 100]));
        events.push(event(tick + 480, 0x80, 0, &[pitch, 0]));
    }
    events.sort_by_key(|e| e.tick);
    events.push(event(4320, 0xff, 0x2f, &[]));
    let mut track = Track { events };
    cat_smf_editor::voicevox::make_singer(&mut track).unwrap();
    cat_smf_editor::edit::refresh_deltas(&mut track).unwrap();
    for (tick, _, text) in notes {
        let on = track
            .events
            .iter()
            .position(|e| e.tick == tick && note_on(e))
            .unwrap();
        set(&mut track, on, text).unwrap();
    }
    let mut tempo = Track {
        events: vec![event(0, 0xff, 0x51, &[0x07, 0xa1, 0x20])],
    };
    cat_smf_editor::edit::refresh_deltas(&mut tempo).unwrap();
    Music {
        format: 1,
        division: 480,
        tracks: vec![tempo, track],
    }
}

pub(super) fn singing(root: &std::path::Path, sing: crate::play::sung::prerender::Sing) -> App {
    let mut app = app(song());
    app.voicevox.prerender.target = Some((50126, root.into()));
    app.voicevox.prerender.sing = Some(sing);
    app.select_track(1);
    app
}

pub(super) fn tracks(app: &App) -> String {
    format!("{:?}", app.loaded.as_ref().unwrap().music.tracks)
}

/// `(tick, lyric)` of every lyric event of track 1.
pub(super) fn lyrics(app: &App) -> Vec<(u64, String)> {
    app.loaded.as_ref().unwrap().music.tracks[1]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == LYRIC)
        .map(|e| (e.tick, String::from_utf8(e.data.clone()).unwrap()))
        .collect()
}

pub(super) fn owned(list: &[(u64, &str)]) -> Vec<(u64, String)> {
    list.iter().map(|&(t, s)| (t, s.to_string())).collect()
}

pub(super) fn on_note(app: &mut App, tick: u64) {
    let loaded = app.loaded.as_ref().unwrap();
    let event = loaded.music.tracks[1]
        .events
        .iter()
        .position(|e| e.tick == tick && note_on(e))
        .unwrap();
    let row = loaded.display_rows[1].event_row(event).unwrap();
    crate::cursor::moves::place(app, row);
}

pub(super) fn assert_looping(app: &App) {
    assert_eq!(app.playback.repeat, Some(MEASURE));
    assert!(transport::timeline::running(app));
}

/// Runs the palette command found by `q` and checks that the measure loops with the input open.
fn begin(app: &mut App, ctx: &egui::Context, q: &str) {
    open(app, ctx);
    query(app, ctx, q);
    let found = listed(app);
    eprintln!("query {q:?}: {found:?}");
    assert_eq!(found.first().map(String::as_str), Some(LABEL));
    press(app, ctx, Key::Enter);
    assert!(!app.palette.is_open());
    frame(app, ctx, vec![]);
    let editing = app.voicevox.lyrics.as_ref().unwrap();
    assert_eq!(editing.replacing.range, MEASURE);
    assert_eq!(app.track_view.cursor_tick, Some(MEASURE.0));
    assert_looping(app);
}

#[test]
fn the_palette_loops_the_measure_and_each_input_overwrites_its_lyrics_one_mora_per_note() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    on_note(&mut app, 2400);
    frame(&mut app, &ctx, vec![]);
    let untouched = tracks(&app);
    assert!(!app.undo.available());

    for q in ["voicevox", "歌詞", "measure"] {
        open(&mut app, &ctx);
        query(&mut app, &ctx, q);
        let found = listed(&mut app);
        eprintln!("query {q:?}: {found:?}");
        assert!(found.iter().any(|label| label == LABEL));
        press(&mut app, &ctx, Key::Escape);
    }
    begin(&mut app, &ctx, "1小節の歌詞");
    assert_eq!(tracks(&app), untouched);

    // The first input overwrites the sung note and removes the lyric without a note.
    frame(&mut app, &ctx, vec![egui::Event::Text("どん".into())]);
    assert_eq!(
        lyrics(&app),
        owned(&[(480, "あ"), (1920, "ど"), (2400, "ん"), (3840, "お")])
    );
    assert_looping(&app);
    // A chord's tick counts once; moras past the measure's notes stay out of measure 3.
    frame(&mut app, &ctx, vec![egui::Event::Text("ぐりころ".into())]);
    let typed = owned(&[
        (480, "あ"),
        (1920, "ど"),
        (2400, "ん"),
        (2880, "ぐ"),
        (3360, "り"),
        (3840, "お"),
    ]);
    assert_eq!(lyrics(&app), typed);
    assert_looping(&app);

    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert_eq!(lyrics(&app), typed);
    assert_looping(&app);
    assert_eq!(app.track_view.cursor_tick, Some(MEASURE.1));
    settle(&mut app, &ctx);
    assert!(SUNG.load(Ordering::SeqCst) > 0);
    assert!(app.voicevox.error.is_none());

    app.undo_music().unwrap();
    assert!(!app.undo.available());
    assert_eq!(tracks(&app), untouched);
}

#[test]
fn cancel_restores_the_measure_lyrics_and_keeps_looping() {
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    on_note(&mut app, 3360);
    frame(&mut app, &ctx, vec![]);
    let untouched = tracks(&app);
    begin(&mut app, &ctx, "1小節の歌詞");
    frame(&mut app, &ctx, vec![egui::Event::Text("きゃり".into())]);
    let got = lyrics(&app);
    assert!(got.contains(&(1920, "きゃ".into())) && got.contains(&(2400, "り".into())));
    press(&mut app, &ctx, Key::Escape);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert_eq!(tracks(&app), untouched);
    assert!(!app.undo.available());
    assert_looping(&app);
    settle(&mut app, &ctx);
}

#[test]
fn a_measure_with_one_note_takes_every_mora_on_notes_added_on_60_and_undo_removes_them() {
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    // Measure 3 holds only the quarter note sung "お".
    on_note(&mut app, 3840);
    frame(&mut app, &ctx, vec![]);
    let untouched = tracks(&app);
    open(&mut app, &ctx);
    query(&mut app, &ctx, "1小節の歌詞");
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("どんぐり".into())]);
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    let events = &app.loaded.as_ref().unwrap().music.tracks[1].events;
    let notes: Vec<_> = events
        .iter()
        .filter(|e| note_on(e) && e.tick >= 3840)
        .map(|e| (e.tick, e.data[0]))
        .collect();
    assert_eq!(notes, [(3840, 57), (4320, 60), (4800, 60), (5280, 60)]);
    assert_eq!(
        lyrics(&app)[3..],
        owned(&[(3840, "ど"), (4320, "ん"), (4800, "ぐ"), (5280, "り")])
    );
    settle(&mut app, &ctx);
    app.undo_music().unwrap();
    assert_eq!(tracks(&app), untouched);
}

fn enter() -> egui::Event {
    egui::Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn the_enter_that_converts_the_ime_text_leaves_the_lyrics_open() {
    use egui::{Event::Ime, ImeEvent};
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = singing(root.path(), sing);
    let ctx = egui::Context::default();
    on_note(&mut app, 2400);
    frame(&mut app, &ctx, vec![]);
    begin(&mut app, &ctx, "1小節の歌詞");
    frame(
        &mut app,
        &ctx,
        vec![
            Ime(ImeEvent::Enabled),
            Ime(ImeEvent::Preedit("どん".into())),
        ],
    );
    // Windows sends the key before the conversion, sometimes one frame earlier.
    frame(&mut app, &ctx, vec![enter()]);
    frame(
        &mut app,
        &ctx,
        vec![
            enter(),
            Ime(ImeEvent::Commit("どん".into())),
            Ime(ImeEvent::Disabled),
        ],
    );
    let editing = app.voicevox.lyrics.as_ref().unwrap();
    assert_eq!(editing.text, "どん");
    assert_looping(&app);

    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.lyrics.is_none());
    assert!(lyrics(&app).contains(&(1920, "ど".into())));
    assert_eq!(app.track_view.cursor_tick, Some(MEASURE.1));
    assert_looping(&app);
    settle(&mut app, &ctx);
}
