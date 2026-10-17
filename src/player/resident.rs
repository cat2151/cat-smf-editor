//! Plays what the resident cat-plugin-player chose: one plugin slot per message,
//! written into the song the same way as a returned selection.
use crate::app::App;
use crate::player::selection::{store_master, store_track};
use cat_smf_editor::{
    chain_settings::PluginState,
    resident_ipc::{self, Open, Selected, Sender, ToPlayer},
    score::instrument::{Patch as Settings, ResolvedLayer},
};
use std::path::PathBuf;

/// The browser the editor last asked for, and the song it was asked for.
#[derive(Clone, Debug)]
pub struct Target {
    pub open: Open,
    /// The patch goes to every track with notes.
    pub bulk: bool,
    pub original: PathBuf,
}

/// Reads the shared state, applies it, and tells the player the result.
///
/// A selection made for another browser or song than `target` is refused:
/// the editor may have asked for another browser before this one arrived.
pub fn answer(
    app: &mut App,
    selected: Selected,
    target: Option<&Target>,
    sender: &mut Sender,
) -> std::io::Result<()> {
    let result = check(app, &selected, target)
        .and_then(|bulk| {
            let state = resident_ipc::read_state(&selected.state)
                .map_err(|error| format!("Could not read the plugin state: {error}"))?;
            Ok((bulk, state))
        })
        .and_then(|(bulk, state)| {
            let mut slot = PluginState {
                reference: selected.reference.map(|r| *r),
                plugin: selected.plugin,
                state,
                preset_name: selected.preset_name,
                bypass: false,
            };
            // The form a reload resolves to, so that the remembered state is found again.
            let reference = {
                use cat_smf_editor::patch_roots::{portable, resolve};
                let roots = &mut app
                    .patch_roots
                    .lookup(app.player_selection.executable.clone());
                let offered = cat_smf_editor::score::layer_track::reference(&slot)?;
                resolve(&portable(&offered, roots)?, roots)?
            };
            slot.plugin = reference.plugin.clone();
            slot.reference = Some(reference.clone());
            app.state_cache.remember(&reference, slot.state.clone())?;
            // A new master drops the running voice, so the position is taken first.
            let resume = crate::play::selection_playback::suspend(app);
            cat_smf_editor::resident_log::log(
                "editor",
                format_args!("apply bulk={bulk} playing={}", resume.is_some()),
            );
            let result = if bulk {
                crate::player::bulk::accept(
                    app,
                    &Settings {
                        layers: vec![ResolvedLayer {
                            instrument: Some(slot),
                            settings: Default::default(),
                        }],
                        ..Default::default()
                    },
                )
            } else {
                apply(app, selected.target, slot)
            };
            if let Some(resume) = resume {
                crate::play::selection_playback::restore(app, resume);
            }
            result
        });
    cat_smf_editor::resident_log::log("editor", format_args!("answer {result:?}"));
    sender.send(&ToPlayer::Applied(result))
}

/// Whether the selection goes to every track with notes.
fn check(app: &App, selected: &Selected, target: Option<&Target>) -> Result<bool, String> {
    if let Open::Layer { track, layer, song } = selected.target {
        let bulk = target.is_some_and(|t| t.bulk);
        if app.song_id != song
            || (!bulk && app.track != track)
            || !app
                .loaded
                .as_ref()
                .and_then(|l| l.patches.get(track))
                .is_some_and(|p| p.layers.iter().any(|l| l.settings.id == layer))
        {
            return Err("Layer selection target no longer exists or is active".into());
        }
    }
    let original = &app.loaded.as_ref().ok_or("No SMF is loaded")?.original;
    target
        .filter(|t| t.open == selected.target && t.original == *original)
        .map(|t| t.bulk && matches!(t.open, Open::Patch { .. } | Open::Layer { .. }))
        .ok_or_else(|| {
            "Selection target changed while the browser was open; result discarded".into()
        })
}

/// A patch replaces the instrument; an effect replaces or adds one chain slot.
pub fn apply(app: &mut App, target: Open, slot: PluginState) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    match target {
        Open::Layer {
            track,
            layer: id,
            song,
        } => {
            if song != app.song_id {
                return Err("Song changed".into());
            }
            let mut settings = loaded.patches.get(track).ok_or("No such track")?.clone();
            let layer = settings
                .layers
                .iter_mut()
                .find(|l| l.settings.id == id)
                .ok_or("Layer was removed")?;
            layer.instrument = Some(slot);
            settings.validate()?;
            store_track(app, track, &settings)
        }
        Open::Patch { track } => {
            let mut settings = loaded.patches.get(track).ok_or("No such track")?.clone();
            let layer = settings.layers.first_mut().ok_or("Track has no layers")?;
            layer.instrument = Some(slot);
            settings.validate()?;
            store_track(app, track, &settings)
        }
        Open::Effect { track, slot: at } => {
            let mut settings = loaded.patches.get(track).ok_or("No such track")?.clone();
            place(&mut settings.effects, at, slot)?;
            settings.validate()?;
            store_track(app, track, &settings)
        }
        Open::MasterEffect { slot: at } => {
            if loaded.music.format != 1 {
                return Err("Master effects editing requires SMF format 1".into());
            }
            let mut master = app.audition.master().clone();
            place(&mut master.effects, at, slot)?;
            master.validate()?;
            store_master(app, master)
        }
    }
}

/// `None` targets the last effect, or adds the first one. One past the end adds.
fn place(
    effects: &mut Vec<PluginState>,
    at: Option<usize>,
    slot: PluginState,
) -> Result<(), String> {
    let at = at.unwrap_or(effects.len().saturating_sub(1));
    if effects.iter().enumerate().any(|(index, effect)| {
        index != at
            && effect.plugin.format == slot.plugin.format
            && effect.plugin.id == slot.plugin.id
    }) {
        return Err(format!(
            "{} is already connected in another slot",
            slot.plugin.name
        ));
    }
    match at.cmp(&effects.len()) {
        std::cmp::Ordering::Less => effects[at] = slot,
        std::cmp::Ordering::Equal => effects.push(slot),
        std::cmp::Ordering::Greater => return Err("No such effect slot".into()),
    }
    Ok(())
}

#[cfg(test)]
#[path = "resident_tests.rs"]
mod tests;
