use super::*;
use std::{io::Read, path::PathBuf};

#[test]
fn palette_finds_project_save_and_escape_keeps_document() {
    use crate::view::palette::tests as palette;
    let mut app = palette::app();
    let _wording = palette::worded(
        &mut app,
        include_str!("../../../../config/command-palette.toml"),
    );
    let ctx = eframe::egui::Context::default();
    palette::open(&mut app, &ctx);
    palette::query(&mut app, &ctx, "save uapmdz project");
    assert_eq!(palette::listed(&mut app), ["file.save-uapmdz"]);
    palette::frame(
        &mut app,
        &ctx,
        vec![eframe::egui::Event::Key {
            key: eframe::egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert!(!app.palette.is_open());
    assert!(app.load_error.is_none());
}

pub(super) fn contents(bytes: Vec<u8>, name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut result = vec![];
    zip.by_name(name).unwrap().read_to_end(&mut result).unwrap();
    result
}

#[test]
fn prepared_events_layers_state_and_document_survive_export() {
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let mut app = crate::view::palette::tests::app();
    let plugin = cat_smf_editor::host::PluginKey {
        format: "CLAP".into(),
        id: "test.synth".into(),
        name: "Test".into(),
        vendor: String::new(),
        bundle_path: "X:/plugins/test.clap".into(),
    };
    let state = cat_smf_editor::chain_settings::PluginState {
        plugin,
        state: vec![1, 2, 3],
        reference: None,
        preset_name: None,
        bypass: false,
    };
    for patch in &mut app.loaded.as_mut().unwrap().patches {
        patch.layers[0].instrument = Some(state.clone());
    }
    let loaded = app.loaded.as_mut().unwrap();
    let mut layer = loaded.patches[1].layers[0].clone();
    layer.settings.id = 2;
    layer.settings.octave = 1;
    layer.settings.volume_db = -6.0206;
    layer.settings.solo = true;
    loaded.patches[1].layers.push(layer);
    crate::play::track_volume::write(&mut loaded.music.tracks[1], 0.5);
    let original = cat_smf_editor::smf_io::encode(&loaded.music).unwrap();
    let dirty = app.saved.dirty();
    let archive = bytes(&app).unwrap();
    let project: serde_json::Value =
        serde_json::from_slice(&contents(archive.clone(), "project.uapmd")).unwrap();
    assert_eq!(project["tracks"].as_array().unwrap().len(), 4);
    let graph: serde_json::Value =
        serde_json::from_slice(&contents(archive.clone(), "graphs/1.json")).unwrap();
    assert_eq!(graph["nodes"][0]["parameters"]["gain"], 0.0);
    let graph: serde_json::Value =
        serde_json::from_slice(&contents(archive.clone(), "graphs/2.json")).unwrap();
    assert!((graph["nodes"][0]["parameters"]["gain"].as_f64().unwrap() - 0.5).abs() < 0.00001);
    assert_eq!(project["tracks"][2]["volume"], 0.5);
    let clip = cat_smf_editor::ffi::parse(&contents(archive.clone(), "clips/2.mid")).unwrap();
    let notes: Vec<_> = clip.tracks[0]
        .events
        .iter()
        .filter(|e| e.status < 0xf0)
        .map(|e| (e.tick, e.status, e.data.clone()))
        .collect();
    assert_eq!(notes, [(0, 0x90, vec![72, 100]), (480, 0x80, vec![72, 0])]);
    assert_eq!(
        contents(archive.clone(), "plugin_states/2.state"),
        state.state
    );
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
    assert_eq!(app.saved.dirty(), dirty);
    let work = tempfile::tempdir().unwrap();
    let path = work.path().join("project.uapmdz");
    export::write(&path, b"old").unwrap();
    export::write(&path, &archive).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), archive);
}

#[test]
fn song_export_uses_arrangement_instead_of_stored_pattern_coordinates() {
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let (app, _, _, _, _) = crate::view::patterns::workspace_tests::verification_song();
    let prepared = crate::play::arrangement::prepare(&app, None).unwrap();
    let archive = bytes(&app).unwrap();
    let expected: usize = prepared
        .parts
        .iter()
        .map(|(_, p, s)| s.events.len() * p.layers.len())
        .sum();
    let mut actual = 0;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        if entry.name().starts_with("clips/") && entry.name() != "clips/master.mid" {
            let mut data = vec![];
            entry.read_to_end(&mut data).unwrap();
            let m = cat_smf_editor::ffi::parse(&data).unwrap();
            actual += m.tracks[0]
                .events
                .iter()
                .filter(|e| e.status < 0xf0)
                .count();
        }
    }
    assert_eq!(actual, expected);
}

#[test]
fn tempo_changes_and_channel_controls_keep_source_ticks() {
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let event = |tick, status, meta_type, data| Event {
        tick,
        delta: 0,
        status,
        meta_type,
        data,
    };
    let master = finish(
        vec![
            event(0, 0xff, 0x51, vec![0x09, 0x27, 0xc0]),
            event(240, 0xff, 0x51, vec![0x06, 0x1a, 0x80]),
            event(0, 0xff, 0x58, vec![3, 2, 24, 8]),
        ],
        960,
    )
    .unwrap();
    let notes = finish(
        vec![
            event(0, 0x92, 0, vec![60, 93]),
            event(120, 0xb2, 0, vec![64, 127]),
            event(360, 0xe2, 0, vec![0, 70]),
            event(720, 0x82, 0, vec![60, 0]),
            event(960, 0xb2, 0, vec![64, 0]),
        ],
        960,
    )
    .unwrap();
    let music = cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![master.clone(), notes.clone()],
    };
    let app = crate::view::tests::app(music);
    let archive = bytes(&app).unwrap();
    let restored_master =
        cat_smf_editor::ffi::parse(&contents(archive.clone(), "clips/master.mid")).unwrap();
    let restored_notes = cat_smf_editor::ffi::parse(&contents(archive, "clips/0.mid")).unwrap();
    let values = |track: &Track| {
        track
            .events
            .iter()
            .map(|e| (e.tick, e.status, e.meta_type, e.data.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(values(&restored_master.tracks[0]), values(&master));
    assert_eq!(values(&restored_notes.tracks[0]), values(&notes));
}

#[test]
#[ignore = "requires UAPMD_TEST_PROJECT and UAPMD_TEST_EXPORT; produces a test-owned archive for repl integration"]
fn real_project_export_fixture() {
    let _library = unsafe { libloading::Library::new(env!("UAPMD_SHIM_DLL")).unwrap() };
    let root = cat_smf_editor::storage_paths::directory().unwrap();
    let table: cat_smf_editor::patch_roots::Table =
        serde_json::from_slice(&std::fs::read(root.join("cache/patch-roots-v1.json")).unwrap())
            .unwrap();
    let _roots = crate::player::patch_roots::Assumed::with_table(table);
    let mut app = App::empty();
    app.audition = crate::play::audition::Audition::disabled();
    app.open_mid_and_play(PathBuf::from(
        std::env::var_os("UAPMD_TEST_PROJECT").unwrap(),
    ));
    crate::view::opening::tests::finish(&mut app, &eframe::egui::Context::default());
    assert!(app.load_error.is_none(), "{:?}", app.load_error);
    if std::env::var_os("UAPMD_TEST_LAYERED").is_some() {
        let loaded = app.loaded.as_mut().unwrap();
        let patch = loaded
            .patches
            .iter_mut()
            .find(|p| !p.effects.is_empty() && p.kit.is_none() && p.drums.is_none())
            .expect("fixture needs a track effect");
        let mut layer = patch.layers[0].clone();
        layer.settings.id = patch.layers.iter().map(|l| l.settings.id).max().unwrap() + 1;
        layer.settings.octave = -1;
        layer.settings.volume_db = -6.0206;
        patch.layers.push(layer);
        let mut bypassed = patch.effects[0].clone();
        bypassed.bypass = true;
        patch.effects.push(bypassed);
        app.audition
            .set_master(cat_smf_editor::master_effects::Settings {
                effects: vec![patch.effects[0].clone()],
                ..Default::default()
            });
    }
    export::write(
        &PathBuf::from(std::env::var_os("UAPMD_TEST_EXPORT").unwrap()),
        &bytes(&app).unwrap(),
    )
    .unwrap();
}
