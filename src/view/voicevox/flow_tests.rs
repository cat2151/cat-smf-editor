//! 新規SMF から、キーとパレットだけで ずんだもん に どんぐりころころ を歌わせるまで。
use crate::app::App;
use crate::view::{
    edit_menu::lyric_tests::{settle, wav},
    palette::tests::{frame, listed, open, press, query},
};
use cat_smf_editor::model::{Music, Track};
use cat_smf_editor::voicevox::{is_singer, score::from_track};
use eframe::egui::{self, Key, Modifiers};
use std::sync::Mutex;

pub(super) const SINGER: &str = "Score: VOICEVOX: 現在のtrackの歌声を選ぶ";
pub(super) const LYRICS: &str = "Score: VOICEVOX: 1小節の歌詞を差し替え";
pub(super) const DONGURI: [&str; 8] = ["ど", "ん", "ぐ", "り", "こ", "ろ", "こ", "ろ"];

pub(super) fn key(app: &mut App, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
    frame(
        app,
        ctx,
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
    );
    // Work deferred behind the busy modal runs on the next pass.
    frame(app, ctx, vec![]);
}

/// `Ctrl+N` from an editor singing with `sing` (the configured engine when `None`) into a temporary WAV
/// cache.
pub(super) fn new_song(
    port: u16,
    root: &std::path::Path,
    sing: Option<crate::play::sung::prerender::Sing>,
) -> (App, egui::Context) {
    let mut app = crate::view::tests::app(Music {
        format: 1,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    });
    // Nothing open yet, as when the editor starts without a song.
    app.loaded = None;
    app.voicevox.prerender.target = Some((port, root.into()));
    app.voicevox.prerender.sing = sing;
    if sing.is_some() {
        app.voicevox.catalog.fixture = Some(vec![Default::default()]);
    }
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    key(&mut app, &ctx, Key::N, Modifiers::CTRL | Modifiers::COMMAND);
    assert_eq!(app.loaded.as_ref().unwrap().name, "untitled.mid");
    (app, ctx)
}

/// Runs the palette command listed as `label` among the `voicevox` results.
pub(super) fn palette(app: &mut App, ctx: &egui::Context, label: &str) {
    open(app, ctx);
    query(app, ctx, "voicevox");
    let listed = listed(app);
    let at = listed.iter().position(|l| l == label).expect(label);
    for _ in 0..at {
        press(app, ctx, Key::ArrowDown);
    }
    press(app, ctx, Key::Enter);
    assert!(!app.palette.is_open());
    if label == SINGER {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
        while app.voicevox.catalog.voices.is_empty() {
            frame(app, ctx, vec![]);
            assert!(
                std::time::Instant::now() < deadline,
                "{:?}",
                app.voicevox.catalog.error
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        frame(
            app,
            ctx,
            vec![egui::Event::Text("ずんだもん ノーマル".into())],
        );
        press(app, ctx, Key::Enter);
        assert!(app.voicevox.singer.is_none());
    }
}

/// The lyrics of the measure at `tick`.
pub(super) fn lyrics(app: &mut App, ctx: &egui::Context, tick: u64, text: &str) {
    crate::cursor::moves::at_tick(app, tick);
    palette(app, ctx, LYRICS);
    assert!(app.voicevox.lyrics.is_some());
    frame(app, ctx, vec![]);
    frame(app, ctx, vec![egui::Event::Text(text.into())]);
    press(app, ctx, Key::Enter);
    assert!(app.voicevox.lyrics.is_none());
}

/// `Space` and the frames until the playback starts; returns the singer's track.
pub(super) fn play(app: &mut App, ctx: &egui::Context) -> usize {
    press(app, ctx, Key::Space);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.playback.pending.is_some() {
        assert!(std::time::Instant::now() < deadline);
        frame(app, ctx, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(app.playback.active.is_some());
    assert!(app.voicevox.error.is_none(), "{:?}", app.voicevox.error);
    let music = &app.loaded.as_ref().unwrap().music;
    (0..music.tracks.len())
        .find(|&t| is_singer(&music.tracks[t]))
        .expect("a singer")
}

/// `(key, lyric)` of each sung note in the engine's request body.
fn sung(json: &str) -> Vec<(u64, String)> {
    let body: serde_json::Value = serde_json::from_str(json).unwrap();
    body["notes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| Some((n["key"].as_u64()?, n["lyric"].as_str()?.to_owned())))
        .collect()
}

/// The last playback cued the singer's take note by note, from the score the engine sang last.
fn assert_sung(app: &App, track: usize, last: &str) -> Vec<(u64, String)> {
    let take = app
        .audition
        .takes
        .iter()
        .find(|(t, _)| *t == track)
        .map(|(_, take)| take.clone())
        .expect("the singer's take");
    let score = from_track(&app.loaded.as_ref().unwrap().music, track).unwrap();
    assert_eq!(score.json(), last);
    let notes = sung(last);
    assert_eq!(take.voiced(), notes.len());
    notes
}

#[test]
fn from_a_new_song_the_palette_alone_makes_zundamon_sing_donguri_on_notes_added_on_60() {
    static SUNG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.lock().unwrap().push(json.to_owned());
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(50126, root.path(), Some(sing));

    // `voicevox` offers what to do next, not only the engine setting.
    open(&mut app, &ctx);
    query(&mut app, &ctx, "voicevox");
    let offered = listed(&mut app);
    assert!(offered.iter().any(|l| l == SINGER), "{offered:?}");
    assert!(offered.iter().any(|l| l == LYRICS), "{offered:?}");
    assert!(
        offered.iter().any(|l| l.contains("VOICEVOX engine")),
        "{offered:?}"
    );
    press(&mut app, &ctx, Key::Escape);

    palette(&mut app, &ctx, SINGER);
    lyrics(&mut app, &ctx, 0, "どんぐり");
    lyrics(&mut app, &ctx, 1920, "ころころ");
    settle(&mut app, &ctx);
    let track = play(&mut app, &ctx);
    let last = SUNG.lock().unwrap().last().cloned().expect("sung");
    let notes = assert_sung(&app, track, &last);
    assert_eq!(
        notes.iter().map(|n| n.1.as_str()).collect::<Vec<_>>(),
        DONGURI
    );
    assert!(notes.iter().all(|n| n.0 == 60), "{notes:?}");
}

#[test]
fn a_melody_typed_as_mml_first_takes_donguri_and_sings_it() {
    static SUNG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn sing(json: &str, rate: u32) -> Result<Vec<u8>, String> {
        SUNG.lock().unwrap().push(json.to_owned());
        Ok(wav(rate))
    }
    let root = tempfile::tempdir().unwrap();
    let (mut app, ctx) = new_song(50126, root.path(), Some(sing));

    palette(&mut app, &ctx, SINGER);
    key(&mut app, &ctx, Key::I, Modifiers::NONE);
    assert!(app.mml_input.open);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("o5l4geefedcc".into())],
    );
    frame(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, Key::Enter);
    frame(&mut app, &ctx, vec![]);
    assert!(!app.mml_input.open);
    lyrics(&mut app, &ctx, 0, "どんぐり");
    lyrics(&mut app, &ctx, 1920, "ころころ");
    settle(&mut app, &ctx);
    let track = play(&mut app, &ctx);
    let last = SUNG.lock().unwrap().last().cloned().expect("sung");
    let notes = assert_sung(&app, track, &last);
    assert_eq!(
        notes.iter().map(|n| n.1.as_str()).collect::<Vec<_>>(),
        DONGURI
    );
    assert_eq!(
        notes.iter().map(|n| n.0).collect::<Vec<_>>(),
        [67, 64, 64, 65, 64, 62, 60, 60]
    );
}
