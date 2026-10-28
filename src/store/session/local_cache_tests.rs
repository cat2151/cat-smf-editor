//! Silent, opt-in verification of the real migrated data through normal startup.
use cat_smf_editor::{model::Music, score::state_cache::Cache, storage_paths};

#[test]
#[ignore = "reads actual local session and caches; no GUI/audio; requires migrated local data"]
fn local_cache_restores_current_document_after_restart_without_player() {
    let root = storage_paths::directory().unwrap();
    let status = std::fs::read(root.join("status.json")).unwrap();
    let startup = std::fs::read(root.join("startup.json")).unwrap();
    // Startup may normalize/re-save status (including newly added default fields).
    // Exercise that path on copies rather than overwriting the user's session.
    let isolated = tempfile::tempdir().unwrap();
    let status_path = isolated.path().join("status.json");
    std::fs::write(&status_path, &status).unwrap();
    std::fs::write(isolated.path().join("startup.json"), &startup).unwrap();
    let config = crate::store::config::Store::local();
    assert!(config.error.is_none(), "{:?}", config.error);
    let expected_session = config.session.clone();
    let mid = config.navigation.last_mid.unwrap();
    let original = std::fs::read(&mid).unwrap();
    let _library = cat_smf_editor::native_library::load_at(&storage_paths::cache(&root)).unwrap();
    let _roots = crate::player::patch_roots::Assumed::with_table(
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap(),
    );
    let parsed = cat_smf_editor::ffi::parse(&original).unwrap();
    let expected = cat_smf_editor::smf_io::encode(&parsed).unwrap();
    let patterns = cat_smf_editor::score::pattern::Catalog::read(&parsed).unwrap();
    let scenes = cat_smf_editor::score::scene::read(&parsed).unwrap();
    println!(
        "Current startup data: {} bytes, {} tracks, {} patterns, {} scenes, {} Song blocks",
        original.len(),
        parsed.tracks.len(),
        patterns.patterns.len(),
        scenes.scenes.len(),
        scenes.song.len()
    );
    let mut previous = None;
    for _ in 0..2 {
        let mut app = crate::view::tests::app(Music {
            format: 1,
            division: 480,
            tracks: vec![],
        });
        app.loaded = None;
        app.config = crate::store::config::Store::open(status_path.clone());
        app.state_cache = Cache::new(Some(root.clone()));
        app.player_selection.executable = Some("X:/missing/player.exe".into());
        app.open_initial(None);
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        if let Some(expected) = &expected_session {
            let actual = app.session().unwrap();
            assert_eq!(actual.track, expected.track);
            assert_eq!(actual.note_mode, expected.note_mode);
            assert_eq!(actual.mbt, expected.mbt);
            assert_eq!(actual.tick, expected.tick);
            assert_eq!(actual.split, expected.split);
            assert_eq!(actual.split_active, expected.split_active);
        }
        let loaded = app.loaded.as_ref().unwrap();
        assert_eq!(loaded.original, std::fs::canonicalize(&mid).unwrap());
        assert_eq!(
            cat_smf_editor::smf_io::encode(&loaded.music).unwrap(),
            expected
        );
        assert_eq!(
            cat_smf_editor::score::scene::read(&loaded.music).unwrap(),
            scenes
        );
        let restored = cat_smf_editor::score::pattern::Catalog::read(&loaded.music).unwrap();
        assert_eq!(
            format!("{:?}", restored.patterns),
            format!("{:?}", patterns.patterns)
        );
        let states: Vec<_> = loaded.patches.iter().map(|p| p.saved().unwrap()).collect();
        if let Some(expected) = &previous {
            assert_eq!(&states, expected);
        }
        previous = Some(states);
        assert!(!app.saved.dirty());
        app.saved.save().unwrap(); // Unchanged live SMF uses the normal no-op save path.
        app.state_cache.save().unwrap();
    }
    assert_eq!(std::fs::read(&mid).unwrap(), original);
    assert_eq!(std::fs::read(root.join("status.json")).unwrap(), status);
    assert_eq!(std::fs::read(root.join("startup.json")).unwrap(), startup);
    assert!(!root.join("patch-states-v1").exists());
    assert!(!root.join("native").exists());
}
