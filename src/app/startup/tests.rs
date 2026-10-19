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
