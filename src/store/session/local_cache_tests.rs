//! Silent, opt-in verification of the real migrated data through normal startup.
use cat_smf_editor::{model::Music, score::state_cache::Cache, storage_paths};

#[test]
#[ignore = "reads actual local session and caches; no GUI/audio; requires migrated local data"]
fn local_cache_restores_current_document_after_restart_without_player() {
    let root = storage_paths::directory().unwrap();
    let status = std::fs::read(root.join("status.json")).unwrap();
    let config = crate::store::config::Store::local();
    assert!(config.error.is_none(), "{:?}", config.error);
    let mid = config.navigation.last_mid.unwrap();
    let original = std::fs::read(&mid).unwrap();
    let _library = cat_smf_editor::native_library::load_at(&storage_paths::cache(&root)).unwrap();
    let mut previous = None;
    for _ in 0..2 {
        let mut app = crate::view::tests::app(Music {
            format: 1,
            division: 480,
            tracks: vec![],
        });
        app.loaded = None;
        app.config = crate::store::config::Store::local();
        app.state_cache = Cache::new(Some(root.clone()));
        app.player_selection.executable = Some("X:/missing/player.exe".into());
        app.open_initial(None);
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        let loaded = app.loaded.as_ref().unwrap();
        assert_eq!(loaded.original, std::fs::canonicalize(&mid).unwrap());
        let states: Vec<_> = loaded.patches.iter().map(|p| p.saved().unwrap()).collect();
        if let Some(expected) = &previous {
            assert_eq!(&states, expected);
        }
        previous = Some(states);
        app.state_cache.save().unwrap();
    }
    assert_eq!(std::fs::read(&mid).unwrap(), original);
    assert_eq!(std::fs::read(root.join("status.json")).unwrap(), status);
    assert!(!root.join("patch-states-v1").exists());
    assert!(!root.join("native").exists());
}
