use crate::app::App;
use crate::view::{
    edit_menu::lyric_tests::{settle, wav},
    palette::tests::{frame, listed, open, press, query},
    tests::app,
};
use cat_smf_editor::model::{Event, Music, Track};
use cat_smf_editor::voicevox::is_singer;
use eframe::egui::{self, Key, Modifiers};
use std::sync::atomic::{AtomicUsize, Ordering};

const LABEL: &str = "Event list: VOICEVOX: 現在のtrackの歌声を選ぶ";

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

/// Track 1 is named `Piano`; track 2 has no Track Name. Each holds one quarter note of its own pitch,
/// so neither score is in the WAV cache when the other was sung.
fn song() -> Music {
    let notes = |name: Option<&str>, pitch: u8| {
        let mut events: Vec<Event> = name
            .map(|name| event(0, 0xff, 0x03, name.as_bytes()))
            .into_iter()
            .collect();
        events.push(event(0, 0x90, 0, &[pitch, 100]));
        events.push(event(480, 0x80, 0, &[pitch, 0]));
        events.push(event(480, 0xff, 0x2f, &[]));
        Track { events }
    };
    let mut music = Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![event(0, 0xff, 0x51, &[0x07, 0xa1, 0x20])],
            },
            notes(Some("Piano"), 60),
            notes(None, 62),
        ],
    };
    for track in &mut music.tracks {
        cat_smf_editor::edit::refresh_deltas(track).unwrap();
    }
    music
}

fn track(app: &App, index: usize) -> &Track {
    &app.loaded.as_ref().unwrap().music.tracks[index]
}

fn offered(app: &mut App, ctx: &egui::Context) -> bool {
    open(app, ctx);
    query(app, ctx, "voicevox");
    let offered = listed(app).iter().any(|label| label == LABEL);
    press(app, ctx, Key::Escape);
    offered
}

fn run(app: &mut App, ctx: &egui::Context) {
    open(app, ctx);
    query(app, ctx, "voicevox");
    assert_eq!(listed(app).first().map(String::as_str), Some(LABEL));
    press(app, ctx, Key::Enter);
    assert!(!crate::view::modal::is_open(app));
    press(app, ctx, Key::Enter);
    assert!(app.voicevox.singer.is_none());
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
fn the_palette_makes_the_current_track_the_singer_once_and_undo_restores_it() {
    static SUNG: AtomicUsize = AtomicUsize::new(0);
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        // Count track renders, independently of the voice picker's eight-note preview.
        let score: serde_json::Value = serde_json::from_str(json).unwrap();
        if score["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["key"].is_number())
            .count()
            == 1
        {
            SUNG.fetch_add(1, Ordering::SeqCst);
        }
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let mut app = app(song());
    app.voicevox.prerender.target = Some((50126, root.path().into()));
    app.voicevox.prerender.sing = Some(sing);
    app.voicevox.catalog.fixture = Some(vec![Default::default()]);
    let ctx = egui::Context::default();
    app.select_track(1);
    frame(&mut app, &ctx, vec![]);
    let named = format!("{:?}", track(&app, 1));

    assert!(offered(&mut app, &ctx));
    run(&mut app, &ctx);
    assert!(is_singer(track(&app, 1)));
    settle(&mut app, &ctx);
    assert_eq!(SUNG.load(Ordering::SeqCst), 1);
    assert!(app.voicevox.error.is_none());
    assert!(offered(&mut app, &ctx));
    undo(&mut app, &ctx);
    assert_eq!(format!("{:?}", track(&app, 1)), named);
    assert!(offered(&mut app, &ctx));

    // A track without a Track Name gets one at its head.
    app.select_track(2);
    frame(&mut app, &ctx, vec![]);
    let unnamed = format!("{:?}", track(&app, 2));
    let count = track(&app, 2).events.len();
    run(&mut app, &ctx);
    assert!(is_singer(track(&app, 2)));
    assert_eq!(track(&app, 2).events.len(), count + 1);
    settle(&mut app, &ctx);
    assert_eq!(SUNG.load(Ordering::SeqCst), 2);
    undo(&mut app, &ctx);
    assert_eq!(format!("{:?}", track(&app, 2)), unnamed);
}

#[test]
fn without_a_song_the_command_is_not_offered() {
    let mut app = app(song());
    app.loaded = None;
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    assert!(!offered(&mut app, &ctx));
}
