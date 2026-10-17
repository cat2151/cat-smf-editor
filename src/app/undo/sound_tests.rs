use crate::{app::App, view::note_edit::tests::chord_song};
use cat_smf_editor::{
    chain_settings::PluginState, host::PluginKey, patch_reference::Source,
    score::instrument::Patch, smf_io,
};
use eframe::egui;
use std::path::PathBuf;

struct Fixture {
    app: App,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cat-smf-sound-undo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        crate::player::patch_roots::Assumed::set_root(&root);
        Self {
            app: crate::view::tests::app(chord_song()),
            root,
        }
    }

    fn slot(&mut self, name: &str, byte: u8) -> PluginState {
        let plugin = self.root.join("fixture.clap");
        let preset = self.root.join(format!("{name}.preset"));
        if !plugin.exists() {
            std::fs::write(&plugin, b"fixture").unwrap();
        }
        std::fs::write(&preset, [byte]).unwrap();
        let mut slot = crate::test_support::referenced(PluginState {
            reference: None,
            plugin: PluginKey {
                format: "CLAP".into(),
                id: "fixture".into(),
                name: "fixture".into(),
                vendor: "fixture".into(),
                bundle_path: plugin.to_string_lossy().into_owned(),
            },
            state: vec![byte],
            preset_name: Some(name.into()),
            bypass: false,
        });
        slot.reference.as_mut().unwrap().source = Source::Catalog {
            path: preset.clone(),
            file: preset,
        };
        self.app
            .state_cache
            .remember(slot.reference.as_ref().unwrap(), slot.state.clone())
            .unwrap();
        slot
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let root = self.root.canonicalize().unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn bytes(app: &App) -> Vec<u8> {
    smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap()
}

fn press(app: &mut App) {
    let ctx = egui::Context::default();
    crate::view::note_edit::tests::render(
        app,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
    );
}

#[test]
fn undo_rebuilds_instruments_layers_effect_order_and_master_from_smf() {
    let mut f = Fixture::new();
    let a = f.slot("piano", 1);
    let b = f.slot("organ", 2);
    let delay = f.slot("delay", 3);
    let reverb = f.slot("reverb", 4);
    let mut patch = Patch::default();
    patch.layers[0].instrument = Some(a.clone());
    patch.effects = vec![delay.clone(), reverb.clone()];
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    let piano = bytes(&f.app);
    patch.layers[0].instrument = Some(b);
    let mut second = patch.layers[0].clone();
    second.settings.id += 1;
    second.settings.octave = -1;
    second.settings.volume_db = -6.0;
    patch.layers.push(second);
    patch.effects.reverse();
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    let layered = bytes(&f.app);
    crate::player::selection::store_master(
        &mut f.app,
        cat_smf_editor::master_effects::Settings {
            effects: vec![
                reverb,
                cat_smf_editor::chain_settings::PluginState {
                    bypass: true,
                    ..delay
                },
            ],
            ..Default::default()
        },
    )
    .unwrap();
    let full = bytes(&f.app);
    crate::view::effects::open(&mut f.app, crate::view::effects::Target::Track(1), 0);
    crate::view::effects::tests::press(
        &mut f.app,
        &egui::Context::default(),
        egui::Key::Delete,
        egui::Modifiers::NONE,
    );
    f.app.undo_music().unwrap();
    assert_eq!(bytes(&f.app), full);
    assert_eq!(
        f.app.loaded.as_ref().unwrap().patches[1].effects,
        patch.effects
    );
    assert_eq!(f.app.audition.master().effects.len(), 2);
    assert!(f.app.audition.master().effects[1].bypass);
    f.app.undo_music().unwrap();
    assert_eq!(bytes(&f.app), layered);
    assert!(f.app.audition.master().effects.is_empty());
    f.app.track = 1;
    crate::view::layers::open(&mut f.app);
    press(&mut f.app);
    assert!(f.app.layer_editor.open);
    assert_eq!(bytes(&f.app), piano);
    let restored = &f.app.loaded.as_ref().unwrap().patches[1];
    assert_eq!(restored.layers.len(), 1);
    assert_eq!(
        restored.layers[0].instrument.as_ref().unwrap().state,
        a.state
    );
    assert_eq!(
        restored
            .effects
            .iter()
            .map(|s| s.preset_name.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["delay", "reverb"]
    );
}

#[test]
fn failed_cache_rebuild_preserves_document_settings_audio_and_history() {
    let mut f = Fixture::new();
    let a = f.slot("piano", 1);
    let mut patch = Patch::default();
    patch.layers[0].instrument = Some(a.clone());
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    patch.layers[0].instrument = None;
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    let before = bytes(&f.app);
    crate::play::playback::start(&mut f.app, crate::play::playback::Mode::AllTracks);
    assert!(f.app.playback.pending.is_some());
    std::fs::remove_file(&a.plugin.bundle_path).unwrap();
    assert!(f.app.undo_music().is_err());
    assert_eq!(bytes(&f.app), before);
    assert_eq!(f.app.loaded.as_ref().unwrap().patches[1], patch);
    assert!(f.app.playback.pending.is_some());
    assert!(f.app.undo.available());
    std::fs::write(&a.plugin.bundle_path, b"fixture").unwrap();
    f.app
        .state_cache
        .remember(a.reference.as_ref().unwrap(), a.state.clone())
        .unwrap();
    f.app.undo_music().unwrap();
    assert_eq!(
        f.app.loaded.as_ref().unwrap().patches[1].layers[0]
            .instrument
            .as_ref()
            .unwrap()
            .state,
        a.state
    );
    assert!(f.app.playback.pending.is_none());
    assert!(f.app.playback.active.is_none());
}

#[test]
fn mixer_drag_is_one_undo_step_and_mixer_keyboard_can_undo() {
    let mut f = Fixture::new();
    let original = bytes(&f.app);
    for gain in [0.9, 0.8, 0.7, 0.6] {
        crate::play::mixer::volume::change(&mut f.app, 1, gain, true).unwrap();
        assert_eq!(
            crate::play::track_volume::read(&f.app.loaded.as_ref().unwrap().music.tracks[1])
                .unwrap(),
            gain
        );
    }
    f.app.undo.end_volume_gesture();
    f.app.playback.mixer_open = true;
    press(&mut f.app);
    assert_eq!(bytes(&f.app), original);
    assert_eq!(f.app.audition.gain(1), 1.0);
    assert!(!f.app.undo.available());
    assert!(f.app.playback.mixer_open);
}

#[test]
fn bulk_patch_change_is_one_step_and_keeps_prior_history() {
    let mut f = Fixture::new();
    let mut music = chord_song();
    music.tracks.push(music.tracks[1].clone());
    f.app = crate::view::tests::app(music);
    let original = bytes(&f.app);
    let a = f.slot("piano", 1);
    let b = f.slot("organ", 2);
    let mut patch = Patch::default();
    patch.layers[0].instrument = Some(a);
    crate::player::bulk::accept(&mut f.app, &patch).unwrap();
    let piano = bytes(&f.app);
    patch.layers[0].instrument = Some(b);
    crate::player::bulk::accept(&mut f.app, &patch).unwrap();
    f.app.undo_music().unwrap();
    assert_eq!(bytes(&f.app), piano);
    for track in [1, 2] {
        assert_eq!(
            f.app.loaded.as_ref().unwrap().patches[track].layers[0]
                .instrument
                .as_ref()
                .unwrap()
                .state,
            vec![1]
        );
    }
    f.app.undo_music().unwrap();
    assert_eq!(bytes(&f.app), original);
    assert!(!f.app.undo.available());
}

#[test]
fn undone_smf_saves_and_reopens_using_disk_cache_without_a_player() {
    let mut f = Fixture::new();
    f.app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(f.root.clone()));
    let a = f.slot("piano", 1);
    let b = f.slot("organ", 2);
    let path = f.root.join("song.mid");
    std::fs::write(&path, bytes(&f.app)).unwrap();
    f.app.loaded.as_mut().unwrap().original = path.clone();
    f.app.saved.directory = Some(f.root.clone());
    f.app.saved.load(&path).unwrap();
    let mut patch = Patch::default();
    patch.layers[0].instrument = Some(a);
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    crate::play::mixer::volume::set(&mut f.app, 1, 0.5).unwrap();
    let piano = bytes(&f.app);
    patch.layers[0].instrument = Some(b);
    crate::player::selection::store_track(&mut f.app, 1, &patch).unwrap();
    f.app.state_cache.save().unwrap();
    f.app.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(f.root.clone()));
    f.app.undo_music().unwrap();
    assert_eq!(bytes(&f.app), piano);
    f.app.saved.save().unwrap();
    let mut reopened = crate::view::tests::app(chord_song());
    reopened.state_cache = cat_smf_editor::score::state_cache::Cache::new(Some(f.root.clone()));
    reopened.load_mid_unchecked(path);
    assert!(reopened.load_error.is_none(), "{:?}", reopened.load_error);
    assert_eq!(bytes(&reopened), piano);
    assert_eq!(reopened.audition.gain(1), 0.5);
    assert_eq!(
        reopened.loaded.as_ref().unwrap().patches[1].layers[0]
            .instrument
            .as_ref()
            .unwrap()
            .state,
        vec![1]
    );
}
