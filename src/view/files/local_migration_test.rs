//! Opt-in validation of a locally migrated MIDI through the normal startup/restore path.
use cat_smf_editor::{model::Music, smf_io};

#[test]
#[ignore = "requires CAT_SMF_MIGRATION_MID; temporarily exercises real status.json and restores it; silent"]
fn migrated_local_mid_reopens_with_patches_and_restores_after_restart() {
    let root = crate::store::config::directory().unwrap();
    let mid = std::path::PathBuf::from(std::env::var_os("CAT_SMF_MIGRATION_MID").unwrap());
    let status = root.join("status.json");
    struct Restore(std::path::PathBuf, Vec<u8>);
    impl Drop for Restore {
        fn drop(&mut self) {
            std::fs::write(&self.0, &self.1).unwrap();
        }
    }
    let restore = Restore(status.clone(), std::fs::read(&status).unwrap());
    let original = std::fs::read(&mid).unwrap();
    let expected = smf_io::load(&mid).unwrap();
    let empty = || Music {
        format: 1,
        division: 480,
        tracks: vec![],
    };
    let make = || {
        let mut app = crate::view::tests::app(empty());
        app.loaded = None;
        app.config = crate::store::config::Store::local();
        app.saved = crate::store::saved::Document::new(Some(root.clone()));
        app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(root.clone()));
        app
    };
    let mut app = make();
    assert!(app.config.navigation.last_mid.is_none());
    app.open_initial(None);
    assert!(app.loaded.is_none());
    app.open_initial(Some(mid.clone()));
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(
        smf_io::encode(&loaded.music).unwrap(),
        smf_io::encode(&expected).unwrap()
    );
    for (patch, track) in loaded.patches.iter().zip(&expected.tracks) {
        assert_eq!(
            patch.saved().unwrap(),
            cat_smf_editor::instrument_layers::read(
                track,
                &mut crate::player::patch_roots::Assumed
            )
            .unwrap()
        );
    }
    let ctx = eframe::egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| crate::view::show(&mut app, ctx));
    app.save_session();
    let saved_session = app.session();
    drop(app);
    let mut restarted = make();
    restarted.open_initial(None);
    assert!(restarted.load_error.is_none(), "{:?}", restarted.load_error);
    assert_eq!(
        restarted.loaded.as_ref().unwrap().original,
        std::fs::canonicalize(&mid).unwrap()
    );
    assert_eq!(restarted.session(), saved_session);
    assert_eq!(std::fs::read(&mid).unwrap(), original);
    drop(restarted);
    drop(restore);
    assert!(crate::store::config::Store::local()
        .navigation
        .last_mid
        .is_none());
}
