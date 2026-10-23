use crate::view::{
    palette::tests::{frame, press},
    tests::app,
};
use cat_smf_editor::{
    model::{Music, Track},
    voicevox::Voice,
};
use eframe::egui::{self, Key};

fn fixture() -> (crate::app::App, egui::Context, tempfile::TempDir) {
    fixture_singing(sample_wav)
}
fn fixture_singing(
    sing: crate::play::sung::prerender::Sing,
) -> (crate::app::App, egui::Context, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let mut app = app(Music {
        format: 0,
        division: 480,
        tracks: vec![Track { events: vec![] }],
    });
    app.voicevox.catalog.fixture = Some(vec![
        Voice::default(),
        Voice {
            name: "四国めたん".into(),
            speaker_uuid: "metan".into(),
            style_id: 3000,
            style_name: "あまあま".into(),
        },
    ]);
    app.voicevox.prerender.target = Some((0, root.path().into()));
    app.voicevox.prerender.sing = Some(sing);
    let ctx = egui::Context::default();
    super::open(&mut app);
    frame(&mut app, &ctx, vec![]);
    (app, ctx, root)
}

#[test]
fn arrows_confirm_selected_character_and_style_without_renaming_and_escape_is_atomic() {
    let (mut app, ctx, _root) = fixture();
    press(&mut app, &ctx, Key::ArrowDown);
    press(&mut app, &ctx, Key::Enter);
    let track = &app.loaded.as_ref().unwrap().music.tracks[0];
    let voice = Voice::read(track).unwrap();
    assert_eq!(voice.name, "四国めたん");
    assert_eq!(voice.style_name, "あまあま");
    assert_eq!(track.name(), None);
    super::open(&mut app);
    frame(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, Key::Escape);
    assert_eq!(
        Voice::read(&app.loaded.as_ref().unwrap().music.tracks[0]),
        Some(voice)
    );
}

#[test]
fn ime_enter_commits_search_only_then_enter_selects_and_escape_does_not_open_an_editor() {
    let (mut app, ctx, _root) = fixture();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Enabled)],
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("めたん".into()))],
    );
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Ime(egui::ImeEvent::Commit("めたん".into())),
            egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(app.voicevox.singer.is_some());
    assert!(!cat_smf_editor::voicevox::is_singer(
        &app.loaded.as_ref().unwrap().music.tracks[0]
    ));
    press(&mut app, &ctx, Key::Enter);
    assert_eq!(
        Voice::read(&app.loaded.as_ref().unwrap().music.tracks[0])
            .unwrap()
            .name,
        "四国めたん"
    );
    assert!(!app.mml_input.open);
}

#[test]
fn missing_voice_survives_cancel_and_track_metadata_survives_smf_roundtrip() {
    let (mut app, ctx, _root) = fixture();
    let missing = Voice {
        name: "Missing".into(),
        style_id: 99999,
        ..Voice::default()
    };
    Voice::write(
        &mut app.loaded.as_mut().unwrap().music.tracks[0],
        Some(&missing),
    )
    .unwrap();
    press(&mut app, &ctx, Key::Escape);
    let music = &app.loaded.as_ref().unwrap().music;
    let bytes = cat_smf_editor::smf_io::encode(music).unwrap();
    let path = _root.path().join("roundtrip.mid");
    std::fs::write(&path, bytes).unwrap();
    let restored = cat_smf_editor::smf_io::load(&path).unwrap();
    assert_eq!(Voice::read(&restored.tracks[0]), Some(missing));
}

#[test]
fn migemo_filters_character_and_style_with_keyboard_confirmation() {
    let (mut app, ctx, _root) = fixture();
    for query in ["metan", "shikoku ama", "四国 ama", "METAN"] {
        app.voicevox.singer.as_mut().unwrap().query = query.into();
        let indices = super::filtered(&mut app);
        assert_eq!(indices, [1], "{query}");
    }
    press(&mut app, &ctx, Key::Enter);
    assert_eq!(
        Voice::read(&app.loaded.as_ref().unwrap().music.tracks[0])
            .unwrap()
            .style_id,
        3000
    );
}

fn sample_wav(json: &str, rate: u32) -> Result<Vec<u8>, String> {
    let score: serde_json::Value = serde_json::from_str(json).unwrap();
    let notes = score["notes"].as_array().unwrap();
    assert_eq!(
        notes
            .iter()
            .filter_map(|n| n["key"].as_u64())
            .collect::<Vec<_>>(),
        [67, 67, 64, 64, 65, 64, 62, 60]
    );
    assert_eq!(
        notes
            .iter()
            .filter_map(|n| n["lyric"].as_str())
            .collect::<String>(),
        "どんぐりころころ"
    );
    assert_eq!(
        notes
            .iter()
            .filter(|n| n["key"].is_number())
            .map(|n| n["frame_length"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [23, 24, 23, 24, 23, 24, 23, 24]
    );
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(40u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 4).to_le_bytes());
    bytes.extend(4u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(4u32.to_le_bytes());
    bytes.extend([0, 64, 0, 192]);
    Ok(bytes)
}

fn finish_preview(app: &mut crate::app::App, ctx: &egui::Context) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        frame(app, ctx, vec![]);
        if !app.voicevox.catalog.preview.running() && !app.voicevox.catalog.preview.waiting() {
            break;
        }
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn automatic_queue_caches_each_style_clears_wait_labels_and_reopening_plays_without_edits() {
    let (mut app, ctx, root) = fixture();
    let before = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    finish_preview(&mut app, &ctx);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    assert!(app.voicevox.catalog.preview.playback.is_some());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    let output = ctx.run(egui::RawInput::default(), |ctx| {
        crate::view::show(&mut app, ctx)
    });
    for voice in &app.voicevox.catalog.voices {
        assert!(output.shapes.iter().any(
            |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == voice.label())
        ));
    }
    let mut commands = Vec::new();
    super::collect(&app, crate::view::commands::Screen::Singer, &mut commands);
    assert!(!commands
        .iter()
        .any(|c| c.id == "singer-preview" || c.key_label().contains("F5")));
    press(&mut app, &ctx, Key::Escape);
    assert!(app.voicevox.catalog.preview.playback.is_none());
    app.voicevox.prerender.sing = Some(|_, _| panic!("cache missed"));
    super::open(&mut app);
    frame(&mut app, &ctx, vec![]);
    assert!(app.voicevox.catalog.preview.playback.is_some());
    press(&mut app, &ctx, Key::ArrowDown);
    assert!(app.voicevox.catalog.preview.playback.is_some());
    assert!(!app.voicevox.catalog.preview.running());
    assert!(!app.voicevox.catalog.preview.waiting());
    assert!(app.voicevox.catalog.error.is_none());
}

fn slow_sample(json: &str, rate: u32) -> Result<Vec<u8>, String> {
    std::thread::sleep(std::time::Duration::from_millis(100));
    sample_wav(json, rate)
}
#[test]
fn closing_cancels_waiting_reservations_and_never_plays_the_inflight_result() {
    let (mut app, ctx, root) = fixture_singing(slow_sample);
    assert!(app.voicevox.catalog.preview.running());
    assert!(app.voicevox.catalog.preview.waiting());
    press(&mut app, &ctx, Key::Escape);
    finish_preview(&mut app, &ctx);
    assert!(app.voicevox.catalog.preview.playback.is_none());
    assert!(!app.voicevox.catalog.preview.waiting());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}
#[test]
fn a_new_selection_waits_for_its_own_wav_and_plays_automatically_when_ready() {
    let (mut app, ctx, _root) = fixture_singing(slow_sample);
    press(&mut app, &ctx, Key::ArrowDown);
    assert!(app.voicevox.catalog.preview.playback.is_none());
    finish_preview(&mut app, &ctx);
    assert!(app.voicevox.catalog.preview.playback.is_some());
    assert!(app.voicevox.catalog.error.is_none());
}
#[test]
fn cached_selection_plays_even_while_another_voice_is_rendering() {
    let (mut app, ctx, root) = fixture_singing(slow_sample);
    // Seed only the second style while the first is being rendered.
    let voice = app.voicevox.catalog.voices[1].clone();
    let score = cat_smf_editor::voicevox::score::demo(voice);
    cat_smf_editor::voicevox::cache::load_or_sing(root.path(), &score, 48000, sample_wav).unwrap();
    super::preview::reset(&mut app);
    frame(&mut app, &ctx, vec![]);
    press(&mut app, &ctx, Key::ArrowDown);
    assert!(app.voicevox.catalog.preview.running());
    assert!(app.voicevox.catalog.preview.playback.is_some());
    finish_preview(&mut app, &ctx);
    assert!(app.voicevox.catalog.preview.playback.is_some());
    assert!(app.voicevox.catalog.error.is_none());
}

#[test]
fn query_changes_reprioritize_waiting_styles_without_duplicating_the_running_one() {
    let (mut app, ctx, _root) = fixture_singing(|json, rate| {
        std::thread::sleep(std::time::Duration::from_millis(500));
        sample_wav(json, rate)
    });
    let style = Voice {
        style_id: 3002,
        style_name: "あまあま".into(),
        ..Voice::default()
    };
    app.voicevox.catalog.voices.push(style);
    app.voicevox.singer.as_mut().unwrap().query = "metan".into();
    frame(&mut app, &ctx, vec![]);
    let ids = |app: &crate::app::App| {
        app.voicevox
            .catalog
            .preview
            .reservations()
            .iter()
            .map(|v| v.style_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&app), [3000, 3002]);
    app.voicevox.singer.as_mut().unwrap().query = "ずんだもん".into();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(ids(&app), [3002, 3000]);
    press(&mut app, &ctx, Key::Escape);
    finish_preview(&mut app, &ctx);
}

#[test]
fn recent_and_query_choices_survive_restart_and_cancel_does_not_record() {
    let (mut a, ctx, root) = fixture();
    let config = root.path().join("status.json");
    a.config = crate::store::config::Store::open(config.clone());
    // Choose the second character with no query.
    press(&mut a, &ctx, Key::ArrowDown);
    press(&mut a, &ctx, Key::Enter);
    super::open(&mut a);
    assert_eq!(super::filtered(&mut a), vec![1, 0]);
    // A short query learns a different choice from global recency.
    a.voicevox.singer.as_mut().unwrap().query = "n".into();
    let rows = super::filtered(&mut a);
    a.voicevox.singer.as_mut().unwrap().selected = rows.iter().position(|&i| i == 0).unwrap();
    super::confirm(&mut a, &ctx);
    super::open(&mut a);
    a.voicevox.singer.as_mut().unwrap().query = "metan".into();
    super::confirm(&mut a, &ctx);
    let file = root.path().join(crate::view::selection_history::SINGER);
    let bytes = std::fs::read(&file).unwrap();
    let (mut b, ctx, _other) = fixture();
    b.config = crate::store::config::Store::open(config);
    super::open(&mut b);
    assert_eq!(super::filtered(&mut b), vec![1, 0]);
    b.voicevox.singer.as_mut().unwrap().query = "n".into();
    assert_eq!(super::filtered(&mut b), vec![0, 1]);
    press(&mut b, &ctx, Key::Escape);
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    // Confirming the current voice also refreshes its recency.
    super::open(&mut a);
    a.voicevox.singer.as_mut().unwrap().query = "metan".into();
    super::confirm(&mut a, &ctx);
    assert_eq!(
        a.selection_history.singer.rank(
            "metan",
            &super::history::identity(&a.voicevox.catalog.voices[1])
        ),
        (0, 0)
    );
}

#[test]
fn singer_navigation_repeats_clamps_and_respects_filter() {
    let (mut a, c, _root) = fixture();
    a.voicevox.catalog.voices = (0..25)
        .map(|i| Voice {
            name: format!("Voice {i}"),
            speaker_uuid: i.to_string(),
            ..Voice::default()
        })
        .collect();
    for (key, modifiers, expected) in [
        (Key::PageDown, egui::Modifiers::NONE, 10),
        (Key::PageDown, egui::Modifiers::NONE, 20),
        (Key::End, egui::Modifiers::CTRL, 24),
        (Key::PageUp, egui::Modifiers::NONE, 14),
        (Key::Home, egui::Modifiers::CTRL, 0),
    ] {
        frame(
            &mut a,
            &c,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: true,
                modifiers,
            }],
        );
        assert_eq!(a.voicevox.singer.as_ref().unwrap().selected, expected);
    }
    frame(&mut a, &c, vec![egui::Event::Text("no-match".into())]);
    press(&mut a, &c, Key::PageDown);
    assert_eq!(a.voicevox.singer.as_ref().unwrap().selected, 0);
    press(&mut a, &c, Key::Escape);
    assert!(a.voicevox.singer.is_none());
}
