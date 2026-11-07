//! A new process uses normal document/startup restoration, with device audition disabled.
use super::*;
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::Kit,
    patch_reference::PatchReference,
    score::{
        instrument::{Patch, ResolvedLayer},
        pattern::{self, operations},
        state_cache::Cache,
    },
};
#[test]
#[ignore = "requires player-exported CAT_KIT_CONTRACT_FIXTURE; normal saves and headless process restarts, offline only"]
fn installed_kit_process_restart_restore() {
    let fixture = PathBuf::from(std::env::var_os("CAT_KIT_CONTRACT_FIXTURE").unwrap());
    let (reference, mut kit, state): (PatchReference, Kit, Vec<u8>) =
        serde_json::from_slice(&std::fs::read(&fixture).unwrap()).unwrap();
    kit.rows = Some(
        [
            vec![None; 4],
            cat_smf_editor::score::drum_kit::layout::DEFAULT.to_vec(),
        ]
        .concat(),
    );
    let root = fixture.parent().unwrap().join("kit-startup");
    let patch_root = match &reference.source {
        cat_smf_editor::patch_reference::Source::Catalog { file, .. } => file.parent().unwrap(),
        _ => unreachable!(),
    };
    crate::player::patch_roots::Assumed::set_root(patch_root);
    if std::env::var_os("CAT_KIT_STARTUP_CHILD").is_some() {
        let _library = cat_smf_editor::native_library::load_at(&root.join("host")).unwrap();
        let table: cat_smf_editor::patch_roots::Table =
            serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
                .unwrap();
        let _roots = crate::player::patch_roots::Assumed::with_table(table);
        let mut initial = App::empty();
        initial.audition = Audition::disabled();
        initial.saved = store::saved::Document::new(Some(root.clone()));
        initial.state_cache = Cache::new(Some(root.clone()));
        initial.patch_roots = crate::player::patch_roots::Cache::new(Some(root.clone()));
        let mut app = App::bootstrap_with(initial, None, None, Some(root.clone()));
        assert!(app.load_error.is_none(), "{:?}", app.load_error);
        assert!(app.loaded.is_some());
        app.audition.set_test_seconds(0.1);
        assert!(!app.startup_pending(&egui::Context::default()));
        let loaded = app.loaded.as_ref().unwrap();
        let catalog = pattern::Catalog::read(&loaded.music).unwrap();
        assert_eq!(catalog.sources.len(), 1);
        assert_eq!(catalog.patterns.len(), 1);
        assert_eq!(
            cat_smf_editor::score::scene::read(&loaded.music)
                .unwrap()
                .song
                .len(),
            1
        );
        let track = *catalog.sources.iter().next().unwrap();
        assert_eq!(loaded.patches[track].kit, Some(kit));
        assert_eq!(loaded.patches[track].layers.len(), 1);
        assert!(crate::view::drums::is_open(&app));
        let active = app
            .playback
            .active
            .as_ref()
            .expect("Song restored through startup");
        let parts: Vec<_> = active
            .sources
            .iter()
            .map(|(t, seq)| (*t, loaded.patches[*t].clone(), seq.clone()))
            .collect();
        let instruments =
            crate::play::audition::offline::Instruments::open(48000, &parts, &Default::default())
                .unwrap();
        let job = unsafe {
            instruments.job(
                48000,
                parts,
                &vec![1.0; loaded.music.tracks.len()],
                true,
                vec![],
            )
        };
        let audio = job
            .run(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!(audio.iter().all(|v| v.is_finite()));
        assert!(audio.iter().any(|v| v.abs() > 0.00001));
        return;
    }
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(root.join("cache")).unwrap();
    let table = cat_smf_editor::patch_roots::Table {
        plugins: vec![cat_smf_editor::patch_roots::Plugin {
            format: reference.plugin.format.clone(),
            id: reference.plugin.id.clone(),
            bundle_path: reference.plugin.bundle_path.clone(),
            roots: vec![patch_root.to_string_lossy().into_owned()],
        }],
    };
    std::fs::write(
        root.join("cache/patch-roots-v1.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let mut cache = Cache::new(Some(root.clone()));
    cache.remember(&reference, state.clone()).unwrap();
    cache.save().unwrap();
    let patch = Patch {
        kit: Some(kit.clone()),
        layers: vec![ResolvedLayer {
            settings: Default::default(),
            instrument: Some(PluginState {
                plugin: reference.plugin.clone(),
                reference: Some(reference),
                state,
                preset_name: Some("CR-909".into()),
                bypass: false,
            }),
        }],
        ..Default::default()
    };
    let mut music = cat_smf_editor::edit::tracks::new_song();
    for track in &mut music.tracks {
        track.events.retain(|e| {
            !cat_smf_editor::score::note_view::note_on(e)
                && !cat_smf_editor::score::note_view::note_off(e)
        });
    }
    let track = operations::create_source(&mut music).unwrap();
    cat_smf_editor::score::instrument::write(
        &mut music.tracks[track],
        &patch,
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    let id = operations::create(&mut music, track, None).unwrap();
    let context = pattern::editing::Context {
        track,
        pattern: id,
        measure: 1,
        placement: None,
        scene: None,
        standalone: true,
    };
    let view = context.project(&music).unwrap();
    let mut edited = view.clone();
    for n in kit.notes.iter().take(3) {
        cat_smf_editor::score::drum_kit::toggle_step(
            &mut edited,
            track,
            n.note,
            1,
            0,
            kit.velocity(n.note),
        )
        .unwrap();
    }
    music = context.map(&music, &view, &edited).unwrap();
    cat_smf_editor::edit::regenerate(&mut music).unwrap();
    let lane = cat_smf_editor::score::scene::arrangement::add_lane(&mut music, None).unwrap();
    cat_smf_editor::score::scene::arrangement::place(
        &mut music,
        None,
        lane,
        1,
        Some(cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern(id)),
    )
    .unwrap();
    let path = root.join("kit-song.mid");
    let mut document = store::saved::Document::untitled(Some(root.clone()), &music).unwrap();
    document.save_as(&path).unwrap();
    let mut config = Store::open(root.join("status.json"));
    config.navigation.last_mid = Some(path.clone());
    config.session = Some(Session {
        file: "kit-song.mid".into(),
        mbt: None,
        track: track + 1,
        tick: Some(0),
        row: None,
        note_mode: true,
        split: None,
        split_active: 0,
        edited_pattern: Some(id),
        step_containers: vec![],
    });
    config.persist();
    store::startup::save(&root.join("status.json"), Some(path)).unwrap();
    for _ in 0..2 {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::startup::tests::kit::installed_kit_process_restart_restore",
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("CAT_KIT_STARTUP_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
