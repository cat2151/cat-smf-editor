use super::*;
use crate::{
    play::audition::Audition,
    store::config::{Session, Store},
};
use cat_smf_editor::smf_io;

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("song.mid");
    // Keep the intro silent; startup completion must depend on playback, not amplitude.
    let music = cat_smf_editor::model::Music {
        format: 0,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track {
            events: [
                (1920, 0x90, vec![60, 80]),
                (2400, 0x80, vec![60, 0]),
                (3840, 0xff, vec![]),
            ]
            .into_iter()
            .map(|(tick, status, data)| cat_smf_editor::model::Event {
                tick,
                delta: 0,
                status,
                meta_type: if status == 0xff { 0x2f } else { 0 },
                data,
            })
            .collect(),
        }],
    };
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let mut config = Store::open(root.path().join("status.json"));
    config.navigation.last_mid = Some(path.clone());
    config.browser_queries.insert("kept".into(), "organ".into());
    config.session = Some(Session {
        file: "song.mid".into(),
        track: 1,
        mbt: None,
        tick: Some(1920),
        row: Some(0),
        note_mode: false,
        split: None,
        split_active: 0,
    });
    config.persist();
    (root, path)
}

fn boot(root: &std::path::Path, mid: Option<PathBuf>) -> App {
    let mut app = App::empty();
    app.audition = Audition::disabled();
    app.saved = store::saved::Document::new(Some(root.to_owned()));
    App::bootstrap_with(app, None, mid, Some(root.to_owned()))
}

#[test]
fn minimal_restore_plays_before_reading_ui_status_and_restores_without_restarting() {
    let (root, path) = fixture();
    let before = std::fs::read(root.path().join("status.json")).unwrap();
    let midi = std::fs::read(&path).unwrap();
    let mut app = boot(root.path(), None);
    assert!(app.startup.pending);
    assert!(app.loaded.as_ref().unwrap().display_rows.is_empty());
    assert!(app.config.browser_queries.is_empty());
    assert!(app.playback.active.is_some());
    assert_eq!(
        std::fs::read(root.path().join("status.json")).unwrap(),
        before
    );
    let generation = app.generation;
    app.audition.set_test_seconds(0.1);
    assert!(!app.startup_pending(&egui::Context::default()));
    assert!(!app.startup.pending);
    assert_eq!(app.config.browser_queries["kept"], "organ");
    assert!(!app.note_mode);
    assert_eq!(app.cursor_row, Some(0));
    assert_eq!(app.generation, generation);
    assert!(app.playback.active.is_some());
    assert_eq!(app.audition.seconds(), Some(0.1));
    assert!(!app.saved.dirty());
    assert_eq!(std::fs::read(&path).unwrap(), midi);
}

#[test]
fn corrupt_deferred_status_cannot_block_audio_or_be_overwritten_by_defaults() {
    let (root, _) = fixture();
    let status = root.path().join("status.json");
    std::fs::write(&status, b"broken status").unwrap();
    let mut app = boot(root.path(), None);
    assert!(app.playback.active.is_some());
    assert!(app.config.error.is_none());
    app.audition.set_test_seconds(0.1);
    app.startup_pending(&egui::Context::default());
    app.save_session();
    assert!(app.config.error.is_some());
    assert_eq!(std::fs::read(status).unwrap(), b"broken status");
}

#[test]
fn explicit_midi_does_not_read_broken_minimal_status() {
    let (root, path) = fixture();
    std::fs::write(root.path().join("startup.json"), b"broken startup").unwrap();
    let mut app = boot(root.path(), Some(path));
    assert!(app.playback.active.is_some());
    assert!(app.config.error.is_none());
    app.audition.set_test_seconds(0.1);
    app.startup_pending(&egui::Context::default());
    assert!(app.note_mode); // CLI open does not restore the previous editing position.
    app.save_session();
    assert_eq!(
        std::fs::read(root.path().join("startup.json")).unwrap(),
        b"broken startup"
    );
}

#[test]
fn stop_during_first_callback_wait_finishes_restore_and_stops_playback() {
    let (root, _) = fixture();
    let mut app = boot(root.path(), None);
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
            ..Default::default()
        },
        |ctx| {
            assert!(!app.startup_pending(ctx));
        },
    );
    assert!(!app.startup.pending);
    assert!(app.playback.active.is_none());
    assert!(!app.saved.dirty());
}

#[test]
fn interrupted_two_file_save_does_not_apply_another_documents_cursor() {
    let (root, path) = fixture();
    let other = root.path().join("other/song.mid");
    std::fs::create_dir(other.parent().unwrap()).unwrap();
    std::fs::copy(path, &other).unwrap();
    // Simulate startup.json being published before its accompanying UI snapshot.
    store::startup::save(&root.path().join("status.json"), Some(other)).unwrap();
    let mut app = boot(root.path(), None);
    app.audition.set_test_seconds(0.1);
    app.startup_pending(&egui::Context::default());
    assert!(app.note_mode);
    assert_eq!(app.cursor_row, None);
    assert_eq!(app.config.browser_queries["kept"], "organ");
}

#[test]
fn first_frame_shortcut_sees_completed_display_indexes() {
    let (root, _) = fixture();
    let mut app = boot(root.path(), None);
    app.audition.set_test_seconds(0.1);
    let ctx = egui::Context::default();
    let modifiers = egui::Modifiers {
        ctrl: true,
        shift: true,
        command: true,
        ..Default::default()
    };
    let _ = ctx.run(
        egui::RawInput {
            modifiers,
            events: vec![egui::Event::Key {
                key: egui::Key::P,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        },
        |ctx| crate::view::show(&mut app, ctx),
    );
    assert!(!app.startup.pending);
    assert!(app.palette.is_open());
    assert!(!app.loaded.as_ref().unwrap().display_rows.is_empty());
}

#[test]
fn singer_wait_restores_ui_without_audio_timeout_and_paints_first_frame_progress() {
    fn slow_sing(_: &str, _: u32) -> Result<Vec<u8>, String> {
        std::thread::sleep(Duration::from_secs(1));
        Err("test synthesis failure".into())
    }
    let (root, path) = fixture();
    let mut music = smf_io::load(&path).unwrap();
    cat_smf_editor::voicevox::make_singer(&mut music.tracks[0]).unwrap();
    std::fs::write(&path, smf_io::encode(&music).unwrap()).unwrap();
    let configured = PathBuf::from("X:/VOICEVOX/run.exe");
    let mut config = Store::open(root.path().join("status.json"));
    config.voicevox_engine = Some(configured.clone());
    config.persist();
    let mut app = App::empty();
    app.audition = Audition::disabled();
    app.saved = store::saved::Document::new(Some(root.path().into()));
    app.voicevox.prerender.target = Some((0, root.path().join("voice")));
    app.voicevox.prerender.sing = Some(slow_sing);
    let mut app = App::bootstrap_with(app, None, None, Some(root.path().into()));
    assert!(crate::play::sung::prerender::running(&app));
    assert_eq!(app.config.voicevox_engine, Some(configured));
    app.startup.started = Some(Instant::now() - Duration::from_secs(61));
    let ctx = egui::Context::default();
    let output = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
    assert!(!app.startup.pending);
    assert!(app.playback.pending.is_some());
    assert!(app.load_error.is_none());
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::epaint::Shape::Text(text) if text.galley.text() == "Preparing VOICEVOX audio…"
            && text.pos.x < 100.0 && text.pos.y < 100.0)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.playback.pending.is_some() {
        assert!(Instant::now() < deadline);
        transport::advance(&mut app);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(crate::play::sung::prerender::progress(&app).is_none());
    assert!(app.playback.active.is_some());
}

#[test]
fn mascot_toggle_restores_on_and_off_after_restart_and_explicit_file_open() {
    let (root, path) = fixture();
    let mut app = boot(root.path(), None);
    app.audition.set_test_seconds(0.1);
    app.startup_pending(&egui::Context::default());
    assert!(!app.mascot.enabled);
    crate::play::mascot::toggle(&mut app);
    assert!(app.mascot.enabled);
    assert!(app.config.error.is_none());
    drop(app);
    for mid in [None, Some(path.clone())] {
        let mut restarted = boot(root.path(), mid);
        restarted.audition.set_test_seconds(0.1);
        restarted.startup_pending(&egui::Context::default());
        assert!(restarted.mascot.enabled);
        assert!(restarted.config.mascot_enabled);
        assert_eq!(restarted.config.browser_queries["kept"], "organ");
        assert!(restarted.config.error.is_none());
    }
    let mut app = boot(root.path(), None);
    app.audition.set_test_seconds(0.1);
    app.startup_pending(&egui::Context::default());
    crate::play::mascot::toggle(&mut app);
    assert!(!app.mascot.enabled);
    drop(app);
    let mut restarted = boot(root.path(), None);
    restarted.audition.set_test_seconds(0.1);
    restarted.startup_pending(&egui::Context::default());
    assert!(!restarted.mascot.enabled);
    assert!(!restarted.config.mascot_enabled);
}

#[test]
#[ignore = "requires REAL_MASCOT_STATUS_PATH; checks real settings without audio or windows"]
fn real_mascot_status_preserves_settings_across_on_off_and_restarts() {
    let path = PathBuf::from(std::env::var_os("REAL_MASCOT_STATUS_PATH").unwrap());
    let original = std::fs::read(&path).unwrap();
    struct Restore(Vec<(PathBuf, Option<Vec<u8>>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (path, bytes) in &self.0 {
                if let Some(bytes) = bytes {
                    std::fs::write(path, bytes).expect("restore original real settings bytes");
                } else if path.exists() {
                    std::fs::remove_file(path).expect("remove test-created history file");
                }
            }
        }
    }
    let recent_path = path.with_file_name("recent-files.json");
    let _restore = Restore(vec![
        (path.clone(), Some(original.clone())),
        (recent_path.clone(), std::fs::read(recent_path).ok()),
    ]);
    let baseline: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let startup_path = path.with_file_name("startup.json");
    let startup_bytes = std::fs::read(&startup_path).unwrap();
    let mid = PathBuf::from(baseline["navigation"]["last_mid"].as_str().unwrap());
    let midi_bytes = std::fs::read(&mid).unwrap();
    let restore_app = || {
        let mut app = crate::view::tests::app(smf_io::load(&mid).unwrap());
        app.loaded.as_mut().unwrap().original = mid.clone();
        app.loaded.as_mut().unwrap().name = baseline["session"]["file"].as_str().unwrap().into();
        app.config.path = Some(path.clone());
        app.startup.pending = true;
        app.startup.restore_position = true;
        assert!(!app.startup_pending(&egui::Context::default()));
        assert!(app.config.error.is_none());
        app
    };
    let mut app = restore_app();
    assert_eq!(
        app.mascot.enabled,
        baseline["mascot_enabled"].as_bool().unwrap()
    );
    for desired in [false, true] {
        if app.mascot.enabled != desired {
            crate::play::mascot::toggle(&mut app);
        }
        assert!(app.config.error.is_none());
        drop(app);
        app = restore_app();
        assert_eq!(app.mascot.enabled, desired);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (key, value) in baseline.as_object().unwrap() {
            if key != "mascot_enabled" {
                assert_eq!(&saved[key], value, "unrelated setting changed: {key}");
            }
        }
    }
    drop(app);
    assert_eq!(std::fs::read(startup_path).unwrap(), startup_bytes);
    assert_eq!(std::fs::read(mid).unwrap(), midi_bytes);
}
