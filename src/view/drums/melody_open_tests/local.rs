//! Read-only verification against the current document, without GUI windows or audio.
use super::*;
use cat_smf_editor::{score::pattern::Catalog, storage_paths};

#[test]
#[ignore = "reads the actual local SMF and instrument caches"]
fn current_smf_shows_ordinary_patterns_in_the_placed_grid_after_restart() {
    let root = storage_paths::directory().unwrap();
    let status = std::fs::read(root.join("status.json")).unwrap();
    let startup = std::fs::read(root.join("startup.json")).unwrap();
    let isolated = tempfile::tempdir().unwrap();
    std::fs::write(isolated.path().join("status.json"), &status).unwrap();
    std::fs::write(isolated.path().join("startup.json"), &startup).unwrap();
    let _library = cat_smf_editor::native_library::load_at(&storage_paths::cache(&root)).unwrap();
    let _roots = crate::player::patch_roots::Assumed::with_table(
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap(),
    );
    let config = crate::store::config::Store::local();
    let path = config.navigation.last_mid.unwrap();
    let original = std::fs::read(&path).unwrap();
    for _ in 0..2 {
        let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
        a.loaded = None;
        a.config = crate::store::config::Store::open(isolated.path().join("status.json"));
        a.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(root.clone()));
        a.open_initial(None);
        assert!(a.load_error.is_none(), "{:?}", a.load_error);
        a.layout.spec = crate::store::layout::parse(
            "[root]\nrow = [{ view = \"song\" }, { view = \"step_sequencer\" }]",
        )
        .unwrap();
        let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
        let mut checked = 0;
        for pattern in catalog.patterns.values() {
            if a.loaded.as_ref().unwrap().patches[pattern.track].sequencer() {
                continue;
            }
            a.patterns.navigation.steps.clear();
            crate::view::patterns::library::edit(&mut a, pattern.id).unwrap();
            assert!(is_open(&a) && melody::active(&a));
            let ctx = egui::Context::default();
            for _ in 0..2 {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| pane(&mut a, ui));
                });
            }
            assert!(ctx
                .data(|d| d.get_temp::<egui::Rect>(egui::Id::new("melody-clip")))
                .is_some());
            checked += 1;
        }
        assert!(
            checked > 0,
            "current SMF needs ordinary patterns for this check"
        );
        println!("Rendered {checked} ordinary patterns from the current SMF");
    }
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert_eq!(std::fs::read(root.join("status.json")).unwrap(), status);
    assert_eq!(std::fs::read(root.join("startup.json")).unwrap(), startup);
}
