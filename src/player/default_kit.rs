//! The installed default drum kit, shared by both song creation paths.
use cat_smf_editor::{
    drum_kit::Kit,
    patch_reference::PatchReference,
    score::{
        instrument::{Patch, ResolvedLayer},
        state_cache::Cache,
    },
};
use std::path::PathBuf;

pub(crate) fn load(cache: &mut Cache, player: Option<PathBuf>) -> Result<Patch, String> {
    let (reference, kit) = catalog(player.clone())?;
    reference.validate()?;
    kit.validate()?;
    let instrument = cache.resolve(&reference, |r| super::state_request::generate(r, player))?;
    let patch = Patch {
        kit: Some(kit),
        layers: vec![ResolvedLayer {
            settings: Default::default(),
            instrument: Some(instrument),
        }],
        ..Default::default()
    };
    patch.validate()?;
    Ok(patch)
}

fn catalog(player: Option<PathBuf>) -> Result<(PatchReference, Kit), String> {
    #[cfg(test)]
    if let Some(fixture) = fixture::CATALOG.take() {
        return Ok(fixture);
    }

    let output = super::state_request::headless(player, "--default-drum-kit")?
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) mod fixture {
    use super::*;
    use cat_smf_editor::{drum_kit::Note, host::PluginKey, patch_reference::Source};

    thread_local! {
        pub(super) static CATALOG: std::cell::Cell<Option<(PatchReference, Kit)>> =
            const { std::cell::Cell::new(None) };
    }

    pub(crate) fn provide(cache: &mut Cache, kick: u8) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let bundle = directory.path().join("fixture.clap");
        let file = directory.path().join("default-kit.sfz");
        std::fs::write(&bundle, b"fixture plugin").unwrap();
        std::fs::write(&file, b"fixture patch").unwrap();
        super::super::patch_roots::Assumed::set_root(directory.path());
        let reference = PatchReference {
            plugin: PluginKey {
                format: "CLAP".into(),
                id: "challenge-kit".into(),
                name: "Kit".into(),
                vendor: String::new(),
                bundle_path: bundle.to_string_lossy().into_owned(),
            },
            source: Source::Catalog {
                path: file.clone(),
                file,
            },
            display: "default-kit.sfz".into(),
        };
        cache.remember(&reference, vec![1, 2, 3]).unwrap();
        let kit = Kit {
            layer: 1,
            notes: vec![Note {
                note: kick,
                name: "Kick 2".into(),
                velocity: 100,
            }],
            rows: None,
            muted: vec![],
            swing: Default::default(),
        };
        CATALOG.set(Some((reference, kit)));
        directory
    }
}
