//! Installed catalog, actual Surge states and a fresh disk-cache restoration; no device stream.
use super::*;

#[test]
#[ignore = "requires installed Surge XT, local catalog and sibling release player; offline only"]
fn installed_drums_save_restart_and_render_each_row() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let plugin = cat_smf_editor::chain::default_instrument().unwrap();
    let home = dirs::config_local_dir().unwrap();
    let table: cat_smf_editor::patch_roots::Table = serde_json::from_slice(
        &std::fs::read(home.join("cat-smf-editor/cache/patch-roots-v1.json")).unwrap(),
    )
    .unwrap();
    let installed = table.plugin(&plugin).unwrap();
    crate::player::patch_roots::Assumed::set_root(std::path::Path::new(&installed.roots[0]));
    let player = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cat-plugin-player/target/release/cat-plugin-player.exe");
    assert!(player.is_file());
    let initial = cat_smf_editor::edit::tracks::new_song();
    let mut app = crate::view::tests::app(initial.clone());
    app.player_selection.executable = Some(player);
    app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
    open(&mut app);
    let ctx = egui::Context::default();
    assert_eq!(
        crate::view::commands::owner(&app, &ctx),
        crate::view::commands::Screen::DrumCreate
    );
    let _ = ctx.run(
        egui::RawInput {
            events: [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect(),
            ..Default::default()
        },
        |c| crate::view::commands::route(&mut app, c),
    );
    assert!(app.drum_editor.error.is_none());
    assert_eq!(app.track, 1);
    assert!(!app.drum_editor.confirm_create);
    let track = app.track;
    let mut preserved = app.loaded.as_ref().unwrap().music.clone();
    preserved.tracks.truncate(1);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&preserved).unwrap(),
        cat_smf_editor::smf_io::encode(&initial).unwrap()
    );
    assert!(!app.loaded.as_ref().unwrap().music.tracks[track]
        .events
        .iter()
        .any(cat_smf_editor::score::note_view::note_on));
    for row in 0..4 {
        app.drum_editor.row = row;
        app.drum_editor.field = row + 2;
        activate(&mut app).unwrap();
    }
    app.state_cache.save().unwrap();
    let original = app.loaded.as_ref().unwrap().patches[track].clone();
    let references = cat_smf_editor::instrument_layers::read(
        &app.loaded.as_ref().unwrap().music.tracks[track],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    for (before, after) in original.layers.iter().zip(&references.layers) {
        assert_eq!(
            serde_json::to_value(&before.instrument.as_ref().unwrap().reference).unwrap(),
            serde_json::to_value(&after.patch).unwrap(),
            "SMF restoration must preserve the state-cache identity exactly"
        );
    }
    let file = work.path().join("drums.mid");
    std::fs::write(
        &file,
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
    )
    .unwrap();
    drop(app);
    let mut music = cat_smf_editor::smf_io::load(&file).unwrap();
    music.tracks = vec![music.tracks.remove(track)];
    let mut fresh = cat_smf_editor::score::state_cache::Cache::new(Some(work.path().into()));
    let (restored, _) = crate::app::sound::resolve_track(
        &music.tracks[0],
        Some("X:/missing/player.exe".into()),
        &mut crate::player::patch_roots::Cache::new(None),
        &mut fresh,
    )
    .unwrap();
    assert_eq!(restored.drums, original.drums);
    assert_eq!(restored.saved().unwrap(), original.saved().unwrap());
    for (before, after) in original.layers.iter().zip(&restored.layers) {
        assert!(
            before.instrument.as_ref().unwrap().state == after.instrument.as_ref().unwrap().state,
            "Restoration changed the cached plugin state"
        );
    }
    assert_eq!(
        music.tracks[0]
            .events
            .iter()
            .filter(|e| cat_smf_editor::score::note_view::note_on(e))
            .map(|e| e.data[0])
            .collect::<Vec<_>>(),
        drums::NOTES
    );
    let source = &music.tracks[0].events;
    let tempo = cat_smf_editor::score::audition_timing::TempoMap::new(&music, 0);
    for row in 0..4 {
        let selected: Vec<_> = source
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                (cat_smf_editor::score::note_view::note_on(e) && e.data[0] == drums::NOTES[row])
                    .then_some(i)
            })
            .collect();
        let sequence = crate::play::audio_sequence::Sequence::from_source(
            vec![crate::play::audio_sequence::Note {
                start: tempo.seconds_at(source[selected[0]].tick),
                duration: 0.125,
                pitch: drums::NOTES[row],
                velocity: 100,
            }],
            tempo.seconds_at(source.last().unwrap().tick) + 0.1,
            source,
            &selected,
            &tempo,
            0,
        );
        assert!(sequence.events.iter().all(|e| (e.word >> 16) & 15 == 9));
        let parts = vec![(0, restored.clone(), sequence)];
        let instruments =
            crate::play::audition::offline::Instruments::open(48000, &parts, &Default::default())
                .unwrap();
        let job = unsafe { instruments.job(48000, parts, &[1.0], true, vec![]) };
        let audio = job
            .run(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!(audio.iter().all(|v| v.is_finite()));
        let peak = audio.iter().fold(0.0_f32, |p, v| p.max(v.abs()));
        assert!(peak > 0.00001, "{} failed to sound", drums::NAMES[row]);
        eprintln!(
            "{}: {} / peak={peak}",
            drums::NAMES[row],
            restored.layers[row].label()
        );
    }
}

#[test]
#[ignore = "reads the current local startup SMF; copies and restores it offline without modifying the original"]
fn installed_drums_current_document_preserves_onsets_and_patch_states() {
    let work = tempfile::tempdir().unwrap();
    let _library = cat_smf_editor::native_library::load_at(work.path()).unwrap();
    let home = dirs::config_local_dir().unwrap().join("cat-smf-editor");
    let startup: serde_json::Value =
        serde_json::from_slice(&std::fs::read(home.join("startup.json")).unwrap()).unwrap();
    let original = std::path::PathBuf::from(startup["last_mid"].as_str().unwrap());
    let bytes = std::fs::read(&original).unwrap();
    let copy = work.path().join("current.mid");
    std::fs::write(&copy, &bytes).unwrap();
    let mut plugin = cat_smf_editor::chain::default_instrument().unwrap();
    let table: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(home.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let installed = table.plugin(&plugin).unwrap();
    plugin.bundle_path = installed.bundle_path;
    crate::player::patch_roots::Assumed::set_root(std::path::Path::new(&installed.roots[0]));
    cat_smf_editor::patch_roots::Roots::plugin(
        &mut crate::player::patch_roots::Assumed,
        &plugin,
        false,
    )
    .unwrap();
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    app.player_selection.executable = Some(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../cat-plugin-player/target/release/cat-plugin-player.exe"),
    );
    app.patch_roots = crate::player::patch_roots::Cache::new(Some(home.clone()));
    app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(home.clone()));
    app.saved.directory = Some(work.path().into());
    app.load_mid_unchecked(copy.clone());
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    let track = app
        .loaded
        .as_ref()
        .unwrap()
        .patches
        .iter()
        .position(|p| p.drums.is_some())
        .unwrap();
    app.select_track(track);
    let before = app.loaded.as_ref().unwrap().music.clone();
    let patch = app.loaded.as_ref().unwrap().patches[track].clone();
    open(&mut app);
    assert!(
        app.drum_editor.error.is_none(),
        "{:?}",
        app.drum_editor.error
    );
    let after = &app.loaded.as_ref().unwrap().music;
    for (i, (a, b)) in before.tracks.iter().zip(&after.tracks).enumerate() {
        let retained = |t: &cat_smf_editor::model::Track| {
            t.events
                .iter()
                .filter(|e| {
                    i != track
                        || !(cat_smf_editor::score::note_view::note_off(e)
                            || e.status == 0xff && e.meta_type == 0x2f)
                })
                .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(retained(a), retained(b));
    }
    assert!(app.loaded.as_ref().unwrap().patches[track] == patch);
    let adjusted = cat_smf_editor::smf_io::encode(after).unwrap();
    app.saved.save_confirmed().unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), adjusted);
    app.load_mid_unchecked(copy);
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    app.select_track(track);
    open(&mut app);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        adjusted
    );
    assert!(app.loaded.as_ref().unwrap().patches[track] == patch);
    assert_eq!(std::fs::read(original).unwrap(), bytes);
}
