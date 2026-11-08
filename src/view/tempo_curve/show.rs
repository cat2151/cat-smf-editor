use super::*;
use std::sync::Arc;

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !crate::view::dock::focus::in_dock(app, ctx, crate::view::dock::ViewKind::TempoCurve) {
        run(app, ctx);
    }
}

/// The frame's work around the curve, after its pane (if any) was drawn: in the main window's
/// pass, or in the dock window holding it.
pub(crate) fn run(app: &mut App, ctx: &egui::Context) {
    let docked = docked(app, ctx);
    if !docked && !app.tempo_curve.open {
        suspend(app);
        return;
    }
    if app.loaded.is_none() {
        return;
    }
    if docked {
        // The pane drew the curve during the main window's pass.
        if let Some((prepared, outcome)) = app.tempo_curve.pane.take() {
            finish(app, &prepared, outcome, true);
        }
        return;
    }
    let prepared = prepare(app);
    let mut outcome = body::Outcome::default();
    let mut open = true;
    window::window(ctx, app.config.tempo_curve_maximized)
        .open(&mut open)
        .show(ctx, |ui| outcome = body::show(app, ui, &prepared));
    finish(app, &prepared, outcome, open);
}

/// Draws the curve as the main window's pane; the rest of the frame's work waits for `show`.
pub(crate) fn pane(app: &mut App, ui: &mut egui::Ui) {
    if crate::view::command_input::preview_active(ui.ctx()) || app.loaded.is_none() {
        return;
    }
    let prepared = prepare(app);
    let outcome = body::show(app, ui, &prepared);
    app.tempo_curve.pane = Some((prepared, outcome));
}

/// The curve lives in a pane, except inside a Views preview, which always shows it floating.
fn docked(app: &App, ctx: &egui::Context) -> bool {
    !crate::view::command_input::preview_active(ctx)
        && crate::view::dock::focus::placed(app, crate::view::dock::ViewKind::TempoCurve)
}

/// A hidden dock window drops the curve's gestures, as a closed curve does.
pub(crate) fn suspend(app: &mut App) {
    app.tempo_curve.viewport.suspend();
    app.tempo_curve.repeat_drag = Default::default();
    app.tempo_curve.drag = None;
    app.tempo_curve.draft = None;
}

fn prepare(app: &mut App) -> Arc<cat_smf_editor::model::Music> {
    match trills::prepare(app) {
        Ok(music) => music,
        Err(error) => {
            app.tempo_curve.error = Some(error);
            Arc::new(app.loaded.as_ref().unwrap().music.clone())
        }
    }
}

fn finish(
    app: &mut App,
    prepared: &Arc<cat_smf_editor::model::Music>,
    outcome: body::Outcome,
    open: bool,
) {
    let body::Outcome {
        repeat_action,
        mut audition_tick,
        mut edited,
        mut reference_edit,
        trill_click,
    } = outcome;
    if let Some(tick) = app.tempo_curve.viewport.seek.take() {
        crate::view::timeline_domain::click(
            crate::view::timeline_controls::Screen::Tempo,
            app,
            0,
            tick,
        );
    }
    if trill_click.is_some() {
        audition_tick = None;
        edited = None;
        reference_edit = None;
    }
    if let Some(tick) = audition_tick {
        interaction::audition(app, tick);
    }
    if let Some(action) = repeat_action {
        repeat::apply(app, action);
    }
    if !open {
        app.tempo_curve.open = false;
    }
    if let Some(point) = reference_edit {
        app.tempo_curve.error = app
            .edit_music(|music| {
                *music = (**prepared).clone();
                cat_smf_editor::edit::reference_tempo::set(music, point.tick, point.bpm)
            })
            .err();
    }
    if let Some(curve) = edited {
        let result = app.edit_music(|music| {
            *music = (**prepared).clone();
            tempo_curve::write(music, curve.as_ref())
        });
        app.tempo_curve.error = result.err();
    }
    if let Some((track, event)) = trill_click {
        app.tempo_curve.error =
            crate::view::trill_edit::begin_prepared(app, track, event, prepared).err();
    }
}
