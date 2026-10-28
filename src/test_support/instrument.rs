//! Saved identity and cached state for parameter tests; no native plugin is opened.
use crate::app::App;
use cat_smf_editor::{
    chain_settings::PluginState,
    host::PluginKey,
    patch_reference::{PatchReference, Source},
    patch_roots::{Plugin, Table},
};

pub struct Instrument {
    _roots: crate::player::patch_roots::ActualRoots,
    _files: tempfile::TempDir,
}

pub fn install(app: &mut App, track: usize) -> Instrument {
    let files = tempfile::tempdir().unwrap();
    let bundle = files.path().join("fixture.clap");
    let preset = files.path().join("fixture.preset");
    // Only fingerprinted, never loaded: the shared UI fixture disables audio.
    std::fs::write(&bundle, b"fixture plugin identity").unwrap();
    std::fs::write(&preset, b"fixture preset identity").unwrap();
    let plugin = PluginKey {
        format: "CLAP".into(),
        id: "org.surge-synth-team.surge-xt".into(),
        name: "Parameter fixture".into(),
        vendor: "Test".into(),
        bundle_path: bundle.to_string_lossy().into_owned(),
    };
    let roots = crate::player::patch_roots::Assumed::with_table(Table {
        plugins: vec![Plugin {
            format: plugin.format.clone(),
            id: plugin.id.clone(),
            bundle_path: plugin.bundle_path.clone(),
            roots: vec![files.path().to_string_lossy().into_owned()],
        }],
    });
    let reference = PatchReference {
        plugin: plugin.clone(),
        source: Source::Catalog {
            path: preset.clone(),
            file: preset,
        },
        display: "Fixture".into(),
    };
    app.state_cache.remember(&reference, vec![1]).unwrap();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.patches[track].layers[0].settings.patch = Some(reference.clone());
    loaded.patches[track].layers[0].instrument = Some(PluginState {
        reference: Some(reference),
        plugin,
        state: vec![1],
        preset_name: Some("Fixture".into()),
        bypass: false,
    });
    cat_smf_editor::score::instrument::write(
        &mut loaded.music.tracks[track],
        &loaded.patches[track],
        &mut crate::player::patch_roots::Assumed,
    )
    .unwrap();
    Instrument {
        _roots: roots,
        _files: files,
    }
}
