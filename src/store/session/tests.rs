use crate::{store::config::Store, view::tests::app};
use cat_smf_editor::model::{Event, Music, Track};

fn fixture(division: u16) -> crate::app::App {
    app(Music {
        format: 2,
        division,
        tracks: (0..2)
            .map(|_| Track {
                events: vec![
                    Event {
                        tick: 480,
                        delta: 480,
                        status: 0x90,
                        meta_type: 0,
                        data: vec![60, 100],
                    },
                    Event {
                        tick: 48_000,
                        delta: 47_520,
                        status: 0x80,
                        meta_type: 0,
                        data: vec![60, 0],
                    },
                ],
            })
            .collect(),
    })
}

#[test]
fn restores_empty_beat_and_track_and_reveals_it_without_audition() {
    let mut original = fixture(480);
    original.select_track(1);
    crate::cursor::moves::select(&mut original, 23);
    let saved = original.session().unwrap();
    assert_eq!(saved.track, 2);
    assert_eq!(saved.mbt.as_deref(), Some("21:1:0"));
    let mut restored = fixture(480);
    restored.restore_position(&saved);
    assert_eq!(
        (restored.track, restored.cursor_row, restored.selected_event),
        (1, Some(23), None)
    );
    assert!(restored.reveal_cursor);
    assert!(!restored.reset_scroll);
    let ctx = eframe::egui::Context::default();
    let output = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::vec2(1500.0, 400.0),
            )),
            ..Default::default()
        },
        |ctx| crate::view::show(&mut restored, ctx),
    );
    assert!(output.shapes.iter().any(|s| matches!(&s.shape, eframe::egui::epaint::Shape::Text(t) if t.galley.text().trim() == "21")));
}

#[test]
fn restores_raw_smpte_and_clamps_stale_track_and_row() {
    let mut original = fixture(0xe728);
    original.note_mode = false;
    original.cursor_row = Some(1);
    original.selected_event = Some(1);
    let mut saved = original.session().unwrap();
    assert_eq!(saved.mbt, None);
    let mut restored = fixture(0xe728);
    restored.restore_position(&saved);
    assert!(!restored.note_mode);
    assert_eq!(restored.selected_event, Some(1));
    saved.track = usize::MAX;
    saved.row = Some(usize::MAX);
    saved.tick = Some(u64::MAX);
    restored.restore_position(&saved);
    assert_eq!((restored.track, restored.cursor_row), (1, Some(1)));
}

#[test]
fn stale_row_is_resolved_by_mbt_and_invalid_mbt_does_not_panic() {
    let mut original = fixture(480);
    original.cursor_row = Some(23);
    let mut saved = original.session().unwrap();
    saved.row = Some(usize::MAX);
    let mut restored = fixture(480);
    restored.restore_position(&saved);
    assert_eq!(restored.cursor_row, Some(23));
    saved.mbt = Some(format!("1:1:{}/{}", u128::MAX, u128::MAX));
    restored.restore_position(&saved);
    assert!(restored.cursor_row.is_some());
}

#[test]
fn config_round_trip_missing_corrupt_and_write_error() {
    let root = std::env::temp_dir().join(format!("cat-smf-session-test-{}", std::process::id()));
    let path = root.join("status.json");
    let mut store = Store::open(path.clone());
    assert!(store.session.is_none());
    assert!(store.error.is_none());
    let mut original = fixture(480);
    original.cursor_row = Some(80);
    let saved = original.session().unwrap();
    store.save(saved.clone());
    assert!(store.error.is_none());
    assert_eq!(Store::open(path.clone()).session, Some(saved.clone()));
    let mut changed = saved.clone();
    changed.track = 2;
    store.save(changed.clone());
    assert!(store.error.is_none());
    assert_eq!(Store::open(path.clone()).session, Some(changed));
    std::fs::write(&path, b"broken json").unwrap();
    assert!(Store::open(path.clone()).error.is_some());
    let mut blocked = Store::open(path.join("status.json"));
    blocked.save(saved);
    assert!(blocked.error.is_some());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn missing_previous_file_preserves_saved_session() {
    let mut original = fixture(480);
    original.cursor_row = Some(80);
    let saved = original.session().unwrap();
    let mut restored = fixture(480);
    restored.loaded = None;
    restored.config.session = Some(saved.clone());
    restored.config.navigation.last_mid = Some("X:/missing/previous.mid".into());
    restored.restore_session();
    restored.save_session();
    assert!(restored.load_error.is_some());
    assert_eq!(restored.config.session, Some(saved));
}

#[test]
fn startup_reopens_the_saved_file_and_position() {
    let root = std::env::temp_dir().join(format!("cat-smf-startup-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("日本語.mid");
    std::fs::write(
        &path,
        [
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 1, 224, b'M', b'T', b'r', b'k', 0, 0,
            0, 13, 0, 0x90, 60, 100, 0x83, 0x60, 0x80, 60, 0, 0, 0xff, 0x2f, 0,
        ],
    )
    .unwrap();
    let mut original = fixture(480);
    original.loaded = None;
    original.open_mid(path.clone());
    original.note_mode = false;
    original.cursor_row = Some(1);
    original.selected_event = Some(1);
    original.config = Store::open(root.join("status.json"));
    original.config.navigation.last_mid = Some(std::fs::canonicalize(&path).unwrap());
    original.save_session();
    let mut restored = fixture(480);
    restored.loaded = None;
    restored.config = Store::open(root.join("status.json"));
    restored.restore_session();
    assert_eq!(restored.loaded.as_ref().unwrap().name, "日本語.mid");
    assert_eq!(restored.session(), original.session());
    assert!(restored.reveal_cursor);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(root.join("status.json")).unwrap();
    std::fs::remove_file(root.join("startup.json")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn session_json_is_renamed_and_existing_status_takes_priority() {
    let root = std::env::temp_dir().join(format!("cat-smf-status-rename-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let session = fixture(480).session().unwrap();
    let original = serde_json::to_vec_pretty(&crate::store::config::Status {
        navigation: Default::default(),
        session: Some(session.clone()),
        program_target: Default::default(),
        tempo_curve_maximized: true,
        piano_roll_maximized: true,
        meta_text_visible: true,
        browser_panes: Default::default(),
        browser_queries: Default::default(),
        patch_cursors: Default::default(),
        analysis_window: None,
        analysis_open: false,
    })
    .unwrap();
    std::fs::write(root.join("config.json"), &original).unwrap();
    let mut store = Store::open_directory(root.clone());
    assert!(store.error.is_none());
    assert_eq!(store.path.as_ref().unwrap(), &root.join("status.json"));
    assert_eq!(store.session, Some(session.clone()));
    assert!(!root.join("config.json").exists());
    assert_eq!(std::fs::read(root.join("status.json")).unwrap(), original);
    let mut changed = session;
    changed.track = 2;
    store.save(changed.clone());
    std::fs::write(root.join("config.json"), &original).unwrap();
    assert_eq!(Store::open_directory(root.clone()).session, Some(changed));
    assert_eq!(std::fs::read(root.join("config.json")).unwrap(), original);
    for name in ["status.json", "config.json"] {
        std::fs::remove_file(root.join(name)).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn two_columns_round_trip_through_status_json_and_invalid_pairs_are_dropped() {
    let root = std::env::temp_dir().join(format!("cat-smf-split-test-{}", std::process::id()));
    let path = root.join("status.json");
    let mut original = fixture(480);
    original.select_track(1);
    original.track_view.split = Some([1, 0]);
    let saved = original.session().unwrap();
    assert_eq!(saved.split, Some([2, 1]));
    Store::open(path.clone()).save(saved);
    let reopened = Store::open(path.clone()).session.unwrap();
    let mut restored = fixture(480);
    restored.restore_position(&reopened);
    assert_eq!(
        (restored.track_view.split, restored.track),
        (Some([1, 0]), 1)
    );
    restored.track_view.split = Some([0, 1]);
    for split in [Some([2, 3]), Some([0, 0]), None] {
        let mut stale = reopened.clone();
        stale.split = split;
        restored.restore_position(&stale);
        assert_eq!(restored.track_view.split, None, "{split:?}");
    }
    // Both panes may show the same track; the right one stays focused.
    let mut same = reopened.clone();
    same.split = Some([2, 2]);
    same.split_active = 1;
    restored.restore_position(&same);
    assert_eq!(
        (restored.track_view.split, restored.track_view.active),
        (Some([1, 1]), 1)
    );
    // The focused slot must hold the saved track.
    same.split = Some([2, 1]);
    restored.restore_position(&same);
    assert_eq!(restored.track_view.split, None);
    // A status.json written before two columns were saved still opens.
    std::fs::write(
        &path,
        br#"{"smf_directory":null,"session":{"file":"a.mid","track":1,"mbt":null,"tick":null,"row":null,"note_mode":true}}"#,
    )
    .unwrap();
    assert_eq!(Store::open(path.clone()).session.unwrap().split, None);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn spectrum_window_round_trips_and_invalid_sizes_are_dropped() {
    let root = std::env::temp_dir().join(format!("cat-smf-analysis-{}", std::process::id()));
    let path = root.join("status.json");
    let mut store = Store::open(path.clone());
    store.select_analysis_window([-20.5, 40.0, 700.0, 250.0]);
    assert!(store.error.is_none());
    assert_eq!(
        Store::open(path.clone()).analysis_window,
        Some([-20.5, 40.0, 700.0, 250.0])
    );
    store.select_analysis_window([10.0, 10.0, 0.0, 250.0]);
    assert_eq!(Store::open(path.clone()).analysis_window, None);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}
