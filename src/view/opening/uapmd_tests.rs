use crate::{
    app::App,
    play::{audition::Audition, playback::Mode},
    view::{opening, palette},
};
use cat_smf_editor::{patch_roots::Table, score::state_cache::Cache};
use eframe::egui::{self, Key, Modifiers};

#[test]
#[ignore = "requires UAPMD_TEST_PROJECT and installed project plugins; renders without an audio device"]
fn real_uapmdz_palette_import_autoplays_and_saved_smf_restores_patches() {
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("UAPMD_TEST_PROJECT").unwrap());
    assert_eq!(
        path.canonicalize().unwrap(),
        crate::view::files::uapmd_project_path()
            .unwrap()
            .canonicalize()
            .unwrap()
    );
    let work = tempfile::tempdir().unwrap();
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let table: Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let _roots = crate::player::patch_roots::Assumed::with_table(table);
    let ctx = egui::Context::default();
    let mut app = App::empty();
    app.audition = Audition::disabled();
    app.state_cache = Cache::new(Some(work.path().join("cache-a")));
    let key = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    palette::tests::open(&mut app, &ctx);
    palette::tests::query(&mut app, &ctx, "load uapmdz project");
    palette::tests::frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(!app.palette.is_open() && !opening::pending(&app));
    palette::tests::open(&mut app, &ctx);
    palette::tests::query(&mut app, &ctx, "load uapmdz project");
    palette::tests::frame(&mut app, &ctx, vec![key(Key::Enter)]);
    opening::tests::finish(&mut app, &ctx);
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    assert!(!app.palette.is_open());
    assert!(app.saved.is_untitled());
    assert_eq!(app.playback.last, Mode::AllTracks);
    let loaded = app.loaded.as_ref().unwrap();
    assert_eq!(loaded.music.tracks.len(), 3);
    let original: Vec<_> = loaded
        .patches
        .iter()
        .skip(1)
        .map(|p| p.layers[0].instrument.as_ref().unwrap().clone())
        .collect();
    assert_eq!(
        original.iter().map(|p| p.state.len()).collect::<Vec<_>>(),
        [59868, 171333]
    );
    let prepared = crate::play::transport::prepare(&app, Mode::AllTracks).unwrap();
    assert_eq!(
        prepared
            .parts
            .iter()
            .map(|(_, _, s)| s.notes.len())
            .collect::<Vec<_>>(),
        [6, 2]
    );
    crate::play::audition::offline::native_probe::verify(prepared);
    let saved = work.path().join("imported.mid");
    std::fs::write(
        &saved,
        cat_smf_editor::smf_io::encode(&loaded.music).unwrap(),
    )
    .unwrap();
    drop(app);
    let mut restored = App::empty();
    restored.audition = Audition::disabled();
    restored.state_cache = Cache::new(Some(work.path().join("cache-b")));
    restored.open_mid_and_play(saved);
    opening::tests::finish(&mut restored, &ctx);
    assert!(restored.load_error.is_none(), "{:?}", restored.load_error);
    for (before, after) in original
        .iter()
        .zip(restored.loaded.as_ref().unwrap().patches.iter().skip(1))
    {
        assert_eq!(
            &after.layers[0].instrument.as_ref().unwrap().state,
            &before.state
        );
        assert_eq!(
            &after.layers[0].instrument.as_ref().unwrap().reference,
            &before.reference
        );
    }
    crate::play::audition::offline::native_probe::verify(
        crate::play::transport::prepare(&restored, Mode::AllTracks).unwrap(),
    );
}
