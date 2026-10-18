//! Rebuild disposable playback settings from the authoritative in-memory SMF.
use super::App;
use cat_smf_editor::{model::Music, score::instrument::Patch};

pub(crate) struct Sound {
    pub patches: Vec<Patch>,
    pub master: cat_smf_editor::master_effects::Settings,
    pub volumes: Vec<f32>,
}

impl App {
    pub(super) fn resolve_sound(&mut self, music: &Music) -> Result<Sound, String> {
        let explicit = self.player_selection.executable.clone();
        resolve(
            music,
            explicit,
            &mut self.patch_roots,
            &mut self.state_cache,
        )
    }
}

pub(crate) fn resolve(
    music: &Music,
    explicit: Option<std::path::PathBuf>,
    patch_roots: &mut crate::player::patch_roots::Cache,
    state_cache: &mut cat_smf_editor::score::state_cache::Cache,
) -> Result<Sound, String> {
    let mut roots = patch_roots.lookup(explicit.clone());
    let mut resolve = |reference: &cat_smf_editor::patch_reference::PatchReference| {
        state_cache.resolve(reference, |r| {
            crate::player::state_request::generate(r, explicit.clone())
        })
    };
    let saved = cat_smf_editor::master_references::read(music, &mut roots)?;
    let master = cat_smf_editor::master_effects::Settings {
        effects: saved
            .effects
            .iter()
            .map(|effect| cat_smf_editor::score::layer_track::resolve_effect(effect, &mut resolve))
            .collect::<Result<_, _>>()?,
        ..Default::default()
    };
    let patches = music
        .tracks
        .iter()
        .map(|track| {
            Patch::resolve(
                cat_smf_editor::instrument_layers::read(track, &mut roots)?,
                &mut resolve,
            )
        })
        .collect::<Result<_, _>>()?;
    let volumes = music
        .tracks
        .iter()
        .map(crate::play::track_volume::read)
        .collect::<Result<_, _>>()?;
    Ok(Sound {
        patches,
        master,
        volumes,
    })
}
