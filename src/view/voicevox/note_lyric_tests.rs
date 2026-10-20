use crate::app::App;
use crate::view::{
    edit_menu::lyric_tests::{settle, wav},
    note_edit::{tests::editing, Field},
    palette::tests::{frame, listed, open, press, query},
    tests::app,
};
use cat_smf_editor::edit::lyric::text_at;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui::{self, Key, Modifiers};
use std::sync::atomic::{AtomicUsize, Ordering};

const LABEL: &str = "Edit: Lyrics";

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// Track 1, named `name`, holds two quarter notes.
fn song(name: &str) -> Music {
    let mut events = vec![event(0, 0xff, 0x03, name.as_bytes())];
    for (n, pitch) in [67u8, 64].into_iter().enumerate() {
        let tick = n as u64 * 480;
        events.push(event(tick, 0x90, 0, &[pitch, 100]));
        events.push(event(tick + 480, 0x80, 0, &[pitch, 0]));
    }
    events.sort_by_key(|e| e.tick);
    events.push(event(960, 0xff, 0x2f, &[]));
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xff, 0x51, &[0x07, 0xa1, 0x20])],
            },
            Track { events },
        ],
    };
    if name == "ずんだもん" {
        cat_smf_editor::voicevox::make_singer(&mut music.tracks[1]).unwrap();
    }
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    music
}

fn events(app: &App) -> &[Event] {
    &app.loaded.as_ref().unwrap().music.tracks[1].events
}

/// Puts the cursor on the event at `index` of track 1.
fn on_event(app: &mut App, ctx: &egui::Context, index: usize) {
    let row = app.loaded.as_ref().unwrap().display_rows[1]
        .event_row(index)
        .unwrap();
    crate::cursor::moves::place(app, row);
    frame(app, ctx, vec![]);
    assert_eq!(app.selected_event, Some(index));
}

fn note(app: &App, tick: u64) -> usize {
    events(app)
        .iter()
        .position(|e| e.tick == tick && e.status == 0x90)
        .unwrap()
}

fn offered(app: &mut App, ctx: &egui::Context, q: &str) -> Vec<String> {
    open(app, ctx);
    query(app, ctx, q);
    let found = listed(app);
    press(app, ctx, Key::Escape);
    found
}

fn run(app: &mut App, ctx: &egui::Context) {
    open(app, ctx);
    query(app, ctx, "lyrics");
    let found = listed(app);
    let row = found.iter().position(|label| label == LABEL).unwrap();
    for _ in 0..row {
        press(app, ctx, Key::ArrowDown);
    }
    press(app, ctx, Key::Enter);
    assert!(!app.palette.is_open());
    frame(app, ctx, vec![]);
}

fn undo(app: &mut App, ctx: &egui::Context) {
    frame(
        app,
        ctx,
        vec![egui::Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::CTRL | Modifiers::COMMAND,
        }],
    );
}

#[test]
fn on_a_note_the_palette_offers_lyrics_and_writes_one_mora_that_undo_removes() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        assert!(json.contains("\"lyric\":\"ど\""));
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = app(song("ずんだもん"));
    app.voicevox.prerender.target = Some((50126, root.path().into()));
    app.voicevox.prerender.sing = Some(sing);
    let ctx = egui::Context::default();
    app.select_track(1);
    frame(&mut app, &ctx, vec![]);
    let first = note(&app, 0);
    on_event(&mut app, &ctx, first);
    let untouched = format!("{:?}", events(&app));

    for q in ["lyrics", "歌詞"] {
        let found = offered(&mut app, &ctx, q);
        eprintln!("query {q:?}: {found:?}");
        assert!(found.contains(&LABEL.to_owned()));
    }

    run(&mut app, &ctx);
    assert_eq!(editing(&app), Some((Field::Lyrics, first, String::new())));
    frame(&mut app, &ctx, vec![egui::Event::Text("ど".into())]);
    assert_eq!(editing(&app).unwrap().2, "ど");
    press(&mut app, &ctx, Key::Enter);
    assert_eq!(editing(&app), None);
    assert_eq!(text_at(events(&app), 0).as_deref(), Some("ど"));
    assert_eq!(text_at(events(&app), 480), None);
    settle(&mut app, &ctx);
    assert_eq!(SUNG.load(Ordering::SeqCst), 1);
    assert!(app.voicevox.error.is_none());

    undo(&mut app, &ctx);
    assert_eq!(format!("{:?}", events(&app)), untouched);
}

#[test]
fn off_a_note_the_palette_does_not_offer_lyrics() {
    let mut app = app(song("Piano"));
    let ctx = egui::Context::default();
    app.select_track(1);
    frame(&mut app, &ctx, vec![]);
    // The Track Name at the head of the track is not a note.
    on_event(&mut app, &ctx, 0);
    for q in ["lyrics", "歌詞"] {
        assert!(!offered(&mut app, &ctx, q).contains(&LABEL.to_owned()));
    }
}
