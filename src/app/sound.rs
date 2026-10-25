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
        resolve_tracks(
            music,
            explicit,
            &mut self.patch_roots,
            &mut self.state_cache,
            self.startup.pending,
        )
    }
}

fn resolve_tracks(
    music: &Music,
    explicit: Option<std::path::PathBuf>,
    patch_roots: &mut crate::player::patch_roots::Cache,
    state_cache: &mut cat_smf_editor::score::state_cache::Cache,
    audible_only: bool,
) -> Result<Sound, String> {
    let _span = cat_smf_editor::startup_trace::span("resolve_sound");
    let master = resolve_master(music, explicit.clone(), patch_roots, state_cache)?;
    let mut patches = Vec::new();
    let mut volumes = Vec::new();
    for track in &music.tracks {
        if audible_only
            && !crate::player::bulk::has_notes(track)
            && cat_smf_editor::score::drums::read(track)?.is_none()
        {
            patches.push(Patch::default());
            volumes.push(1.0);
            continue;
        }
        let (patch, volume) = resolve_track(track, explicit.clone(), patch_roots, state_cache)?;
        patches.push(patch);
        volumes.push(volume);
    }
    Ok(Sound {
        patches,
        master,
        volumes,
    })
}

pub(crate) fn resolve_master(
    music: &Music,
    explicit: Option<std::path::PathBuf>,
    patch_roots: &mut crate::player::patch_roots::Cache,
    state_cache: &mut cat_smf_editor::score::state_cache::Cache,
) -> Result<cat_smf_editor::master_effects::Settings, String> {
    let mut roots = patch_roots.lookup(explicit.clone());
    let mut resolve = |reference: &cat_smf_editor::patch_reference::PatchReference| {
        state_cache.resolve(reference, |r| {
            crate::player::state_request::generate(r, explicit.clone())
        })
    };
    let saved = cat_smf_editor::master_references::read(music, &mut roots)?;
    Ok(cat_smf_editor::master_effects::Settings {
        effects: saved
            .effects
            .iter()
            .map(|effect| cat_smf_editor::score::layer_track::resolve_effect(effect, &mut resolve))
            .collect::<Result<_, _>>()?,
        ..Default::default()
    })
}

pub(crate) fn resolve_track(
    track: &cat_smf_editor::model::Track,
    explicit: Option<std::path::PathBuf>,
    patch_roots: &mut crate::player::patch_roots::Cache,
    state_cache: &mut cat_smf_editor::score::state_cache::Cache,
) -> Result<(Patch, f32), String> {
    let mut roots = patch_roots.lookup(explicit.clone());
    let mut patch = Patch::resolve(
        cat_smf_editor::instrument_layers::read(track, &mut roots)?,
        &mut |reference: &cat_smf_editor::patch_reference::PatchReference| {
            state_cache.resolve(reference, |r| {
                crate::player::state_request::generate(r, explicit.clone())
            })
        },
    )?;
    patch.drums = cat_smf_editor::score::drums::read(track)?;
    patch.validate()?;
    Ok((patch, crate::play::track_volume::read(track)?))
}
