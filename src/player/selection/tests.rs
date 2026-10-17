use super::*;
use cat_smf_editor::{
    chain_settings::PluginState,
    host::PluginKey,
    model::{Event, Music, Track},
    smf_io,
};

/// Events other than the chain and master-effect metadata.
pub(super) fn signature(track: &Track) -> Vec<(u64, u8, u8, &[u8])> {
    track
        .events
        .iter()
        .filter(|e| {
            !(e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(chain_settings::PREFIX))
                && !cat_smf_editor::master_effects::owned(e)
                && !e
                    .data
                    .starts_with(cat_smf_editor::master_references::PREFIX)
        })
        .map(|e| (e.tick, e.status, e.meta_type, e.data.as_slice()))
        .collect()
}

pub(super) fn directory(name: &str) -> PathBuf {
    let root = std::env::var_os("EDITOR_SELECTION_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = root.join(format!(
        "editor-selection-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn event(tick: u64, status: u8, meta_type: u8, data: Vec<u8>) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data,
    }
}
pub(super) fn music() -> Music {
    Music {
        format: 1,
        division: 480,
        tracks: vec![
            Track {
                events: vec![
                    event(0, 0xff, 0x51, vec![3, 208, 144]),
                    event(0, 0x90, 0, vec![99, 10]),
                    event(480, 0xff, 0x51, vec![7, 161, 32]),
                    event(960, 0xff, 0x2f, vec![]),
                ],
            },
            Track {
                events: vec![
                    event(0, 0xb3, 0, vec![1, 77]),
                    event(0, 0xff, 0x7f, vec![1, 2, 3]),
                    event(120, 0x93, 0, vec![60, 81]),
                    event(720, 0x83, 0, vec![60, 0]),
                    event(960, 0xff, 0x2f, vec![]),
                ],
            },
        ],
    }
}
pub(super) fn settings() -> Settings {
    let slot = |id: &str| {
        crate::test_support::referenced(PluginState {
            reference: None,
            plugin: PluginKey {
                format: "CLAP".into(),
                id: id.into(),
                name: id.into(),
                vendor: "fixture".into(),
                bundle_path: "X:/plugins/fixture.clap".into(),
            },
            state: vec![1],
            preset_name: Some("Browser choice".into()),
            bypass: false,
        })
    };
    cat_smf_editor::score::instrument::Patch::from(cat_smf_editor::chain_settings::Settings {
        instrument: Some(slot("source")),
        effects: vec![slot("delay"), slot("reverb")],
        bypass: true,
        ..Default::default()
    })
}

#[test]
fn storing_settings_targets_its_track_and_preserves_events_and_backs_up_original() {
    let root = directory("import");
    let original = root.join("original.mid");
    let source = music();
    let bytes = smf_io::encode(&source).unwrap();
    std::fs::write(&original, &bytes).unwrap();
    let mut app = crate::view::tests::app(source.clone());
    app.loaded.as_mut().unwrap().original = original.clone();
    app.saved = crate::store::saved::Document::new(Some(root.join("config")));
    app.saved.load(&original).unwrap();
    app.select_track(0);
    store_track(&mut app, 1, &settings()).unwrap();
    assert_eq!(app.loaded.as_ref().unwrap().patches[0], Settings::default());
    assert_eq!(app.loaded.as_ref().unwrap().patches[1], settings());
    assert_eq!(
        signature(&app.loaded.as_ref().unwrap().music.tracks[0]),
        signature(&source.tracks[0])
    );
    assert_eq!(
        signature(&app.loaded.as_ref().unwrap().music.tracks[1]),
        signature(&source.tracks[1])
    );
    app.saved.save().unwrap();
    let backup = std::fs::read_dir(root.join("config/backup"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(std::fs::read(backup).unwrap(), bytes);
    let saved = app.saved.load(&original).unwrap();
    assert_eq!(
        cat_smf_editor::instrument_layers::read(
            &saved.tracks[1],
            &mut crate::player::patch_roots::Assumed
        )
        .unwrap(),
        settings().saved().unwrap()
    );
}

#[test]
fn discovery_prefers_override_then_adjacent_then_sibling_release_then_path() {
    let root = directory("discovery");
    let editor = root.join("cat-smf-editor/target/debug/cat-smf-editor.exe");
    let sibling = root.join("cat-plugin-player/target/release/cat-plugin-player.exe");
    let adjacent = editor.parent().unwrap().join("cat-plugin-player.exe");
    let in_path = root.join("path dir/cat-plugin-player.exe");
    let explicit = root.join("override dir/player.exe");
    for file in [&editor, &sibling, &in_path, &explicit] {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, []).unwrap();
    }
    let path = std::env::join_paths([in_path.parent().unwrap()]).unwrap();
    assert_eq!(resolve(None, &editor, Some(path.clone())).unwrap(), sibling);
    let release_editor = root.join("cat-smf-editor/target/release/cat-smf-editor.exe");
    assert_eq!(resolve(None, &release_editor, None).unwrap(), sibling);
    std::fs::write(&adjacent, []).unwrap();
    assert_eq!(
        resolve(None, &editor, Some(path.clone())).unwrap(),
        adjacent
    );
    assert_eq!(
        resolve(Some(explicit.clone()), &editor, Some(path.clone())).unwrap(),
        explicit
    );
    assert!(resolve(Some(root.join("missing.exe")), &editor, Some(path.clone())).is_err());
    std::fs::remove_file(adjacent).unwrap();
    std::fs::remove_file(sibling).unwrap();
    assert_eq!(resolve(None, &editor, Some(path.clone())).unwrap(), in_path);
    let installed_editor = root.join("installed/cat-smf-editor.exe");
    assert!(sibling_release(&installed_editor).is_none());
    assert!(sibling_release(&root.join("random/debug/editor.exe")).is_none());
    assert_eq!(
        resolve(None, &installed_editor, Some(path)).unwrap(),
        in_path
    );
}

#[test]
fn missing_player_has_download_guide_and_explicit_invalid_path_is_not_ignored() {
    assert!(resolve(
        Some("X:/missing/player.exe".into()),
        Path::new("X:/editor/editor.exe"),
        None
    )
    .unwrap_err()
    .contains("--player"));
    assert!(resolve(None, Path::new("X:/editor/editor.exe"), None)
        .unwrap_err()
        .contains("not installed"));
    assert_eq!(
        DOWNLOAD,
        "https://github.com/cat2151/cat-plugin-player/releases"
    );
}
