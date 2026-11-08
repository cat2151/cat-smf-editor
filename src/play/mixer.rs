use crate::{
    app::App,
    play::track_volume::{MAX_DB, MIN_DB, STEP_DB},
};
use eframe::egui;

mod body;
pub(crate) mod volume;
pub(crate) use body::Outcome;
/// Chained effects stand apart from the gray instrument and track labels.
pub const EFFECT_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 190, 255);

pub fn effect_name(effect: &cat_smf_editor::chain_settings::PluginState) -> &str {
    effect.preset_name.as_deref().unwrap_or(&effect.plugin.name)
}

/// The chain of the master, or of the mixer track (the current track outside the
/// mixer), with its last effect selected.
pub fn open_effects(app: &mut App, master: bool) {
    let track = if app.playback.mixer_focused() {
        app.playback.mixer_track
    } else {
        app.track
    };
    let target = if master {
        crate::view::effects::Target::Master
    } else {
        crate::view::effects::Target::Track(track)
    };
    crate::view::effects::open(app, target, usize::MAX);
}

pub fn open(app: &mut App) {
    prepare(app);
    focus(app);
}

/// Opens the mixer on the current track without moving the pane focus, as a Views preview does.
pub fn prepare(app: &mut App) {
    app.playback.mixer_track = app.track;
    app.playback.mixer_master = true;
    app.playback.mixer_open = true;
    app.playback.mixer_focus = true;
}

/// Gives the keys to the mixer; a placed mixer takes the pane focus with them.
pub fn focus(app: &mut App) {
    app.playback.mixer_focus = true;
    if placed(app) {
        crate::view::dock::focus::set(app, crate::view::dock::ViewKind::Mixer);
    }
}

fn placed(app: &App) -> bool {
    crate::view::dock::focus::placed(app, crate::view::dock::ViewKind::Mixer)
}

/// Hands keys back to the views; a focused fader would otherwise keep them.
pub fn unfocus(app: &mut App, ctx: &egui::Context) {
    app.playback.mixer_focus = false;
    ctx.memory_mut(|memory| {
        if let Some(id) = memory.focused() {
            memory.surrender_focus(id);
        }
    });
}

#[derive(Clone, Copy)]
pub(crate) enum Operation {
    Previous,
    Next,
    Quieter,
    Louder,
    Effects,
}

pub(crate) fn apply(app: &mut App, operation: Operation) {
    let count = app.loaded.as_ref().map_or(0, |l| l.track_titles.len());
    if count == 0 {
        return;
    }
    app.playback.mixer_track = app.playback.mixer_track.min(count - 1);
    match operation {
        Operation::Previous => {
            if app.playback.mixer_master {
                return;
            }
            if app.playback.mixer_track == 0 {
                app.playback.mixer_master = true;
            }
            app.playback.mixer_track = app.playback.mixer_track.saturating_sub(1);
        }
        Operation::Next => {
            if app.playback.mixer_master {
                app.playback.mixer_master = false;
                app.playback.mixer_track = 0;
            } else {
                app.playback.mixer_track = (app.playback.mixer_track + 1).min(count - 1);
            }
        }
        Operation::Quieter | Operation::Louder => {
            if app.playback.mixer_master {
                return;
            }
            let track = app.playback.mixer_track;
            let delta = if matches!(operation, Operation::Quieter) {
                -STEP_DB
            } else {
                STEP_DB
            };
            let db = (gain_to_db(app.audition.gain(track)) + delta).clamp(MIN_DB, MAX_DB);
            if let Err(e) = volume::set(app, track, crate::play::track_volume::db_to_gain(db)) {
                app.load_error = Some(e);
            }
        }
        Operation::Effects => open_effects(app, app.playback.mixer_master),
    }
}

#[cfg(test)]
pub fn handle_key(app: &mut App, key: egui::Key) -> bool {
    let ctx = egui::Context::default();
    let defs = crate::view::commands::definitions(app, crate::view::commands::Screen::Mixer);
    let Some(c) = defs.into_iter().find(|c| {
        c.keys
            .iter()
            .any(|(k, m)| *k == key && *m == egui::Modifiers::NONE)
    }) else {
        return false;
    };
    crate::view::commands::run_direct(app, &ctx, &c);
    true
}

fn gain_to_db(gain: f32) -> i32 {
    (20.0 * gain.log10())
        .round()
        .clamp(MIN_DB as f32, MAX_DB as f32) as i32
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !crate::view::dock::focus::in_dock(app, ctx, crate::view::dock::ViewKind::Mixer) {
        run(app, ctx);
    }
}

/// The frame's work around the mixer, after its pane (if any) was drawn: in the main window's
/// pass, or in the dock window holding it.
pub(crate) fn run(app: &mut App, ctx: &egui::Context) {
    let preview = crate::view::command_input::preview_active(ctx);
    if !preview && placed(app) {
        // The pane drew the mixer during the main window's pass.
        let outcome = app.playback.mixer_pane.take().unwrap_or_default();
        return finish(app, ctx, outcome, false, true);
    }
    if !app.playback.mixer_open {
        app.undo.end_volume_gesture();
        return;
    }
    let mut open = true;
    let mut outcome = Outcome::default();
    let mut frame = egui::Frame::window(&ctx.style());
    if app.playback.mixer_focus {
        frame.stroke = ctx.style().visuals.selection.stroke;
    }
    let window = if preview {
        crate::view::command_input::preview_window(ctx, "Mixer (m)")
    } else {
        crate::view::presentation::window(
            ctx,
            crate::view::commands::Screen::Mixer,
            egui::Id::new("Mixer (m)"),
            "Mixer (m)",
        )
        .frame(frame)
        .default_width(760.0)
    };
    let window = window
        .open(&mut open)
        .show(ctx, |ui| outcome = body::show(app, ui));
    if let Some(window) = window
        && let Some(press) =
            ctx.input(|i| i.pointer.press_origin().filter(|_| i.pointer.any_pressed()))
    {
        if window.response.rect.contains(press) {
            app.playback.mixer_focus = true;
        } else if ctx
            .layer_id_at(press)
            .is_some_and(|layer| layer.order == egui::Order::Background)
        {
            unfocus(app, ctx);
        }
    }
    app.playback.mixer_open = open;
    finish(app, ctx, outcome, preview, open);
}

/// Draws the mixer as the main window's pane; the rest of the frame's work waits for `show`.
pub(crate) fn pane(app: &mut App, ui: &mut egui::Ui) {
    if crate::view::command_input::preview_active(ui.ctx()) {
        return;
    }
    let outcome = egui::ScrollArea::vertical()
        .id_salt("mixer-pane")
        .show(ui, |ui| body::show(app, ui))
        .inner;
    app.playback.mixer_pane = Some(outcome);
}

fn finish(app: &mut App, ctx: &egui::Context, outcome: Outcome, preview: bool, open: bool) {
    let Outcome {
        chain,
        volume_change,
    } = outcome;
    if let Some((track, gain, dragging)) = volume_change
        && let Err(error) = volume::change(app, track, gain, dragging)
    {
        app.load_error = Some(error);
    }
    if !preview && (!open || !ctx.input(|i| i.pointer.primary_down())) {
        app.undo.end_volume_gesture();
    }
    if let Some((target, slot)) = chain {
        crate::view::effects::open(app, target, slot);
    }
}

#[cfg(test)]
mod tests;
