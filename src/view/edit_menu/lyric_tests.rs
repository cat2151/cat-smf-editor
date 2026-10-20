use crate::app::App;
use crate::view::{
    note_edit::{
        tests::{editing, enter, render},
        Field,
    },
    tests::app,
};
use cat_smf_editor::edit::lyric::text_at;
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const SINGER: usize = 1;

fn key(app: &mut App, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    render(
        app,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect(),
    );
}

fn press(app: &mut App, ctx: &egui::Context, k: egui::Key) {
    key(app, ctx, k, egui::Modifiers::NONE);
}

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// Three quarter notes on a track named `name`.
fn song(name: &str) -> Music {
    let mut events = vec![event(0, 0xff, 0x03, name.as_bytes())];
    for (n, pitch) in [67u8, 64, 64].into_iter().enumerate() {
        let tick = n as u64 * 480;
        events.push(event(tick, 0x90, 0, &[pitch, 100]));
        events.push(event(tick + 480, 0x80, 0, &[pitch, 0]));
    }
    events.sort_by_key(|e| e.tick);
    events.push(event(1440, 0xff, 0x2f, &[]));
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

/// A 16 bit stereo WAV of one silent frame, as the engine would return it.
pub(crate) fn wav(rate: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    for v in [1u16, 2] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 4).to_le_bytes());
    for v in [4u16, 16] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(b"data");
    b.extend_from_slice(&4u32.to_le_bytes());
    b.extend_from_slice(&[0, 0, 0, 0]);
    b
}

/// The singer's track with the cursor on its first note; the engine is a counting fake.
fn singing(
    name: &str,
    root: &std::path::Path,
    sing: crate::play::sung::prerender::Sing,
) -> (App, egui::Context) {
    let mut app = app(song(name));
    app.voicevox.prerender.target = Some((50126, root.into()));
    app.voicevox.prerender.sing = Some(sing);
    let ctx = egui::Context::default();
    app.select_track(SINGER);
    on_note(&mut app, 0);
    render(&mut app, &ctx, vec![]);
    (app, ctx)
}

fn lyrics(music: &Music) -> Vec<(u64, String)> {
    let events = &music.tracks[SINGER].events;
    [0, 480, 960]
        .into_iter()
        .filter_map(|tick| Some((tick, text_at(events, tick)?)))
        .collect()
}

fn note_index(app: &App, tick: u64) -> usize {
    app.loaded.as_ref().unwrap().music.tracks[SINGER]
        .events
        .iter()
        .position(|e| e.tick == tick && e.status == 0x90)
        .unwrap()
}

fn on_note(app: &mut App, tick: u64) {
    let event = note_index(app, tick);
    let row = app.loaded.as_ref().unwrap().display_rows[SINGER]
        .event_row(event)
        .unwrap();
    crate::cursor::moves::place(app, row);
}

/// Lets the background prerender finish, including one queued while it ran.
pub(crate) fn settle(app: &mut App, ctx: &egui::Context) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut idle = 0;
    while idle < 2 {
        assert!(std::time::Instant::now() < deadline);
        render(app, ctx, vec![]);
        if crate::play::sung::prerender::running(app) || crate::view::voicevox::singer::running(app)
        {
            idle = 0;
        } else {
            idle += 1;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn e_l_and_period_type_lyrics_on_two_notes_that_save_undo_and_sing_once_each() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        assert!(json.contains("\"lyric\":\"ど\""));
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = singing("ずんだもん", root.path(), sing);
    let untouched = format!("{:?}", app.loaded.as_ref().unwrap().music.tracks);

    // Esc leaves the song as it was.
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::L);
    assert_eq!(
        editing(&app),
        Some((Field::Lyrics, note_index(&app, 0), String::new()))
    );
    render(&mut app, &ctx, vec![egui::Event::Text("ど".into())]);
    assert_eq!(editing(&app).unwrap().2, "ど");
    press(&mut app, &ctx, egui::Key::Escape);
    assert_eq!(editing(&app), None);
    assert_eq!(
        format!("{:?}", app.loaded.as_ref().unwrap().music.tracks),
        untouched
    );
    assert_eq!(SUNG.load(Ordering::SeqCst), 0);

    press(&mut app, &ctx, egui::Key::E);
    assert!(app.edit_menu.open);
    let drawn = render(&mut app, &ctx, vec![]);
    assert!(drawn.shapes.iter().any(
        |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == "l: Lyrics")
    ));
    press(&mut app, &ctx, egui::Key::L);
    enter(&mut app, &ctx, "ど");
    assert_eq!(editing(&app), None);
    press(&mut app, &ctx, egui::Key::J);
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(
        editing(&app),
        Some((Field::Lyrics, note_index(&app, 480), String::new()))
    );
    enter(&mut app, &ctx, "ん");
    let music = &app.loaded.as_ref().unwrap().music;
    assert_eq!(lyrics(music), vec![(0, "ど".into()), (480, "ん".into())]);
    settle(&mut app, &ctx);
    assert_eq!(SUNG.load(Ordering::SeqCst), 2);
    assert!(app.voicevox.error.is_none());

    // Reopening shows the lyric; confirming it unchanged needs no new WAV.
    on_note(&mut app, 0);
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(
        editing(&app),
        Some((Field::Lyrics, note_index(&app, 0), "ど".into()))
    );
    enter(&mut app, &ctx, "ど");
    settle(&mut app, &ctx);
    assert_eq!(SUNG.load(Ordering::SeqCst), 2);

    let path = root.path().join("lyrics.mid");
    // As if the song had been opened from `path`, so undo keeps writing to the same document.
    app.loaded.as_mut().unwrap().original = path.clone();
    app.saved.save_as(&path).unwrap();
    let reread = cat_smf_editor::smf_io::load(&path).unwrap();
    assert_eq!(lyrics(&reread), vec![(0, "ど".into()), (480, "ん".into())]);

    key(&mut app, &ctx, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(
        lyrics(&app.loaded.as_ref().unwrap().music),
        vec![(0, "ど".into())]
    );
    key(&mut app, &ctx, egui::Key::Z, egui::Modifiers::CTRL);
    assert_eq!(
        format!("{:?}", app.loaded.as_ref().unwrap().music.tracks),
        untouched
    );
}

#[test]
fn a_lyric_on_another_track_name_sings_nothing() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(_: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.fetch_add(1, Ordering::SeqCst);
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = singing("Piano", root.path(), sing);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::L);
    enter(&mut app, &ctx, "ど");
    assert_eq!(
        lyrics(&app.loaded.as_ref().unwrap().music),
        vec![(0, "ど".into())]
    );
    assert!(!crate::play::sung::prerender::running(&app));
    assert_eq!(SUNG.load(Ordering::SeqCst), 0);
}

#[test]
fn period_on_the_next_notes_lyric_row_types_that_notes_lyric() {
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = singing("Piano", root.path(), |_, rate| Ok(wav(rate)));
    on_note(&mut app, 480);
    render(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::L);
    enter(&mut app, &ctx, "ん");
    on_note(&mut app, 0);
    render(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, egui::Key::Period);
    enter(&mut app, &ctx, "ど");
    // From the first note, `j` stops on the second note's lyric row.
    press(&mut app, &ctx, egui::Key::J);
    let loaded = app.loaded.as_ref().unwrap();
    let selected = &loaded.music.tracks[SINGER].events[app.selected_event.unwrap()];
    assert_eq!((selected.tick, selected.meta_type), (480, 0x05));
    press(&mut app, &ctx, egui::Key::Period);
    assert_eq!(
        editing(&app),
        Some((Field::Lyrics, note_index(&app, 480), "ん".into()))
    );
    enter(&mut app, &ctx, "ぐ");
    assert_eq!(
        lyrics(&app.loaded.as_ref().unwrap().music),
        vec![(0, "ど".into()), (480, "ぐ".into())]
    );
}

#[test]
fn a_lyric_confirmed_while_singing_is_sung_after_the_running_job() {
    static OPEN: AtomicBool = AtomicBool::new(false);
    static SUNG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        while !OPEN.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        SUNG.lock().unwrap().push(json.to_owned());
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = singing("ずんだもん", root.path(), sing);
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::L);
    enter(&mut app, &ctx, "ど");
    press(&mut app, &ctx, egui::Key::J);
    press(&mut app, &ctx, egui::Key::Period);
    enter(&mut app, &ctx, "ん");
    assert!(crate::play::sung::prerender::running(&app));
    OPEN.store(true, Ordering::SeqCst);
    settle(&mut app, &ctx);
    let sung = SUNG.lock().unwrap().clone();
    assert_eq!(sung.len(), 2);
    assert!(!sung[0].contains("\"lyric\":\"ん\""));
    assert!(sung[1].contains("\"lyric\":\"ん\""));
}

#[test]
fn a_lyric_the_engine_rejects_is_kept_and_its_error_shown() {
    const REJECTED: &str = "400 Bad Request {\"detail\":\"lyricが不正です: どん\"}";
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = singing("ずんだもん", root.path(), |_, _| Err(REJECTED.into()));
    press(&mut app, &ctx, egui::Key::E);
    press(&mut app, &ctx, egui::Key::L);
    enter(&mut app, &ctx, "どん");
    settle(&mut app, &ctx);
    assert_eq!(
        lyrics(&app.loaded.as_ref().unwrap().music),
        vec![(0, "どん".into())]
    );
    assert_eq!(app.voicevox.error.as_deref(), Some(REJECTED));
    assert!(!app.voicevox.wizard);
}
