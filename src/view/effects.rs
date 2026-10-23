//! Track and master effect chains: order, bypass, removal and the browser entry.
use crate::app::App;
use cat_smf_editor::chain_settings::PluginState;
use eframe::egui;

pub(crate) mod commands;
mod keyboard;
#[cfg(test)]
pub(crate) mod tests;
mod widgets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Track(usize),
    Master,
}

#[derive(Default)]
pub struct State {
    target: Option<Target>,
    selected: usize,
    /// Effects seen last frame; a grown chain selects the effect the browser added.
    count: usize,
    help: bool,
    reveal: bool,
    drag: Option<usize>,
}

impl State {
    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }
    pub fn close_track(&mut self) {
        if matches!(self.target, Some(Target::Track(_))) {
            self.target = None;
        }
    }
}

pub fn open(app: &mut App, target: Target, selected: usize) {
    if let Target::Track(track) = target
        && track != app.track
    {
        app.select_track(track);
    }
    let count = effects(app, target).map_or(0, |e| e.len());
    app.effect_editor = State {
        target: Some(target),
        selected: selected.min(count.saturating_sub(1)),
        count,
        reveal: true,
        ..Default::default()
    };
    app.layer_editor.open = false;
    app.playback.open = false;
    app.help_open = false;
    app.comma_menu.open = false;
    app.edit_menu.open = false;
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.note_edit = None;
    app.trill_edit = None;
    app.track_view.go_open = false;
    app.track_view.split_open = false;
}

fn effects(app: &App, target: Target) -> Option<Vec<PluginState>> {
    match target {
        Target::Track(track) => Some(app.loaded.as_ref()?.patches.get(track)?.effects.clone()),
        Target::Master => app
            .loaded
            .as_ref()
            .map(|_| app.audition.master().effects.clone()),
    }
}

fn store(app: &mut App, target: Target, effects: Vec<PluginState>) -> Result<(), String> {
    match target {
        Target::Track(track) => {
            let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
            let mut patch = loaded.patches.get(track).ok_or("No such track")?.clone();
            patch.effects = effects;
            patch.validate()?;
            crate::player::selection::store_track(app, track, &patch)
        }
        Target::Master => {
            let mut master = app.audition.master().clone();
            master.effects = effects;
            master.validate()?;
            crate::player::selection::store_master(app, master)
        }
    }
}

/// Opens the browser on chain position `slot`; one past the last effect adds.
fn browse(app: &mut App, target: Target, slot: usize) {
    let browser = match target {
        Target::Track(_) => crate::player::selection::Browser::Effect,
        Target::Master => crate::player::selection::Browser::MasterEffect,
    };
    crate::player::selection::open_slot(app, browser, slot);
}

/// One edit of the chain, from keys or the pointer.
enum Action {
    Close,
    Browse(usize),
    Move { from: usize, to: usize },
    Bypass(usize),
    Remove(usize),
    AutoGain,
}

fn apply(app: &mut App, target: Target, action: Action) -> Result<(), String> {
    let Some(mut chain) = effects(app, target) else {
        return Ok(());
    };
    match action {
        Action::Close => {
            app.effect_editor.target = None;
            return Ok(());
        }
        Action::Browse(slot) => {
            browse(app, target, slot.min(chain.len()));
            return Ok(());
        }
        Action::AutoGain => {
            let enabled = app.audition.auto_gain_enabled();
            app.audition.set_auto_gain_enabled(!enabled);
            return Ok(());
        }
        Action::Move { from, to } => {
            if from >= chain.len() || to >= chain.len() || from == to {
                return Ok(());
            }
            let effect = chain.remove(from);
            chain.insert(to, effect);
            app.effect_editor.selected = to;
        }
        Action::Bypass(slot) => {
            let Some(effect) = chain.get_mut(slot) else {
                return Ok(());
            };
            effect.bypass = !effect.bypass;
        }
        Action::Remove(slot) => {
            if slot >= chain.len() {
                return Ok(());
            }
            chain.remove(slot);
            app.effect_editor.selected = slot.min(chain.len().saturating_sub(1));
        }
    }
    app.effect_editor.count = chain.len();
    app.effect_editor.reveal = true;
    store(app, target, chain)
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.effect_editor.is_open() {
        crate::view::commands::route(app, ctx);
    }
    let Some(target) = app.effect_editor.target else {
        return;
    };
    let Some(chain) = effects(app, target) else {
        app.effect_editor.target = None;
        return;
    };
    let state = &mut app.effect_editor;
    if chain.len() > state.count {
        state.selected = chain.len() - 1;
        state.reveal = true;
    }
    state.count = chain.len();
    state.selected = state.selected.min(chain.len().saturating_sub(1));
    let mut actions = Vec::new();
    if app.effect_editor.is_open() {
        actions.extend(widgets::show(app, ctx, target, &chain));
    }
    for action in actions {
        if let Err(error) = apply(app, target, action) {
            app.player_selection.error = Some(error);
            break;
        }
    }
    crate::view::comma_menu::swallow(ctx);
}

pub(crate) fn show_preview(app: &mut App, ctx: &egui::Context) {
    let target = Target::Track(app.track);
    let Some(chain) = effects(app, target) else {
        return;
    };
    let saved = std::mem::take(&mut app.effect_editor);
    app.effect_editor.target = Some(target);
    let _ = widgets::show(app, ctx, target, &chain);
    app.effect_editor = saved;
}
