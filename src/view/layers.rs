//! Track-local layers with a keyboard cursor shared by mouse and keys.
use crate::app::App;
use cat_smf_editor::score::instrument::{Patch, ResolvedLayer};
use eframe::egui;

pub(crate) mod commands;
mod keyboard;
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod palette_tests;
#[cfg(test)]
mod shortcut_tests;
#[cfg(test)]
mod tests;
mod widgets;

#[derive(Default)]
pub struct State {
    pub open: bool,
    selected: Option<u64>,
    field: usize,
    editing: Option<Editing>,
    help: bool,
    reveal: bool,
    automation: Option<u64>,
}

struct Editing {
    text: String,
    focus: bool,
}

pub fn open(app: &mut App) {
    if app
        .loaded
        .as_ref()
        .is_some_and(|l| l.patches[app.track].drums.is_some())
    {
        super::drums::open(app);
        return;
    }
    app.drum_editor.open = false;
    if super::parameter_editor::is_track(app) {
        super::parameter_editor::open(app);
        return;
    }
    let Some(patch) = app.loaded.as_ref().and_then(|l| l.patches.get(app.track)) else {
        return;
    };
    app.layer_editor = State {
        open: true,
        selected: patch.layers.first().map(|l| l.settings.id),
        reveal: true,
        ..Default::default()
    };
    app.playback.mixer_open = false;
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

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

#[cfg(test)]
pub(crate) fn add(patch: &mut Patch) -> Result<(), String> {
    duplicate(patch, patch.layers.len().saturating_sub(1))
}

fn duplicate(patch: &mut Patch, source: usize) -> Result<(), String> {
    patch.validate()?;
    if patch.layers.len() >= 64 {
        return Err("Too many instrument layers".into());
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let floor = patch
        .layers
        .iter()
        .map(|l| l.settings.id)
        .max()
        .unwrap_or(0);
    let old = NEXT
        .try_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |n| n.max(floor).checked_add(1),
        )
        .map_err(|_| "Layer ID exhausted")?;
    let mut layer = patch.layers.get(source).cloned().unwrap_or(ResolvedLayer {
        settings: Default::default(),
        instrument: None,
    });
    layer.settings.id = old.max(floor) + 1;
    patch.layers.push(layer);
    patch.validate()
}

/// One step of `field` (1 = octave, 2 = dB); keys and the wheel share the amounts.
fn nudge(patch: &mut Patch, row: usize, field: usize, up: bool) -> Result<(), String> {
    let mut candidate = patch.clone();
    let Some(layer) = candidate.layers.get_mut(row) else {
        return Ok(());
    };
    match (field, up) {
        (1, true) => layer.settings.octave = layer.settings.octave.saturating_add(1),
        (1, false) => layer.settings.octave = layer.settings.octave.saturating_sub(1),
        (2, true) => layer.settings.volume_db += 6.0,
        (2, false) => layer.settings.volume_db -= 6.0,
        _ => return Ok(()),
    }
    candidate.validate()?;
    *patch = candidate;
    Ok(())
}

impl State {
    pub(crate) fn allows_undo(&self) -> bool {
        !self.open || (self.editing.is_none() && !self.help)
    }
    fn row(&self, patch: &Patch) -> usize {
        patch
            .layers
            .iter()
            .position(|l| Some(l.settings.id) == self.selected)
            .unwrap_or(0)
    }
    fn normalize(&mut self, patch: &Patch) {
        self.selected = patch.layers.get(self.row(patch)).map(|l| l.settings.id);
    }
    fn add(&mut self, patch: &mut Patch) -> Result<(), String> {
        duplicate(patch, self.row(patch))?;
        self.selected = patch.layers.last().map(|l| l.settings.id);
        self.editing = None;
        self.reveal = true;
        Ok(())
    }
    fn remove(&mut self, patch: &mut Patch, id: u64) {
        let row = self.row(patch);
        patch.layers.retain(|l| l.settings.id != id);
        if self.selected == Some(id) {
            self.selected = patch
                .layers
                .get(row.min(patch.layers.len().saturating_sub(1)))
                .map(|l| l.settings.id);
            self.editing = None;
        }
        self.reveal = true;
    }
    fn begin(&mut self, patch: &Patch) {
        let Some(layer) = patch.layers.get(self.row(patch)) else {
            return;
        };
        let text = match self.field {
            1 => layer.settings.octave.to_string(),
            2 => layer.settings.volume_db.to_string(),
            _ => return,
        };
        self.editing = Some(Editing { text, focus: true });
    }
    fn commit(&mut self, patch: &mut Patch) -> Result<(), String> {
        let text = &self
            .editing
            .as_ref()
            .ok_or("No value is being edited")?
            .text;
        let mut candidate = patch.clone();
        let layer = candidate
            .layers
            .get_mut(self.row(patch))
            .ok_or("Layer was removed")?;
        match self.field {
            1 => layer.settings.octave = text.parse().map_err(|_| "Invalid integer octave")?,
            2 => layer.settings.volume_db = text.parse().map_err(|_| "Invalid dB")?,
            _ => return Err("No numeric field selected".into()),
        }
        candidate.validate()?;
        *patch = candidate;
        self.editing = None;
        Ok(())
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.layer_editor.open {
        crate::view::commands::route(app, ctx);
    }
    if !app.layer_editor.open {
        return;
    }
    let Some(mut patch) = app
        .loaded
        .as_ref()
        .and_then(|l| l.patches.get(app.track))
        .cloned()
    else {
        app.layer_editor.open = false;
        return;
    };
    let before = patch.clone();
    app.layer_editor.normalize(&patch);
    let mut choose = None;
    let mut error = None;
    if app.layer_editor.open {
        widgets::show(
            &mut app.layer_editor,
            ctx,
            &mut patch,
            &mut choose,
            &mut error,
            app.player_selection.error.as_deref(),
        );
    }
    if patch != before {
        match patch
            .validate()
            .and_then(|_| crate::player::selection::store_track(app, app.track, &patch))
        {
            Ok(()) => app.player_selection.error = None,
            Err(e) => {
                error = Some(e);
                app.layer_editor.normalize(&before);
            }
        }
    }
    if let Some(error) = error {
        app.player_selection.error = Some(error);
    }
    if let Some(id) = app.layer_editor.automation.take() {
        super::parameter_editor::choose(app, id);
    }
    if let Some(id) = choose {
        crate::player::selection::open_layer(app, id);
    }
    crate::view::comma_menu::swallow(ctx);
}

/// Draw the regular layer editor against an isolated patch and cursor.
pub(crate) fn show_preview(app: &App, ctx: &egui::Context) {
    let Some(mut patch) = app
        .loaded
        .as_ref()
        .and_then(|l| l.patches.get(app.track))
        .cloned()
    else {
        return;
    };
    let mut state = State {
        open: true,
        selected: patch.layers.first().map(|l| l.settings.id),
        ..Default::default()
    };
    widgets::show(&mut state, ctx, &mut patch, &mut None, &mut None, None);
}
