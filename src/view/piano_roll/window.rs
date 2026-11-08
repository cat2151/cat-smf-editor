use super::*;
use crate::view::timeline_repeat as repeat;

pub(crate) fn show(app: &mut App, ctx: &egui::Context) {
    if !crate::view::dock::focus::in_dock(app, ctx, crate::view::dock::ViewKind::PianoRoll) {
        run(app, ctx);
    }
}

/// The frame's work around the roll, after its pane (if any) was drawn: in the main window's
/// pass, or in the dock window holding it.
pub(crate) fn run(app: &mut App, ctx: &egui::Context) {
    if app.piano_roll.generation != app.generation {
        app.piano_roll.drag = None;
        app.piano_roll.drag_pane = None;
        app.piano_roll.range_drag = None;
        app.piano_roll.generation = app.generation;
    }
    chord_timing::validate(app);
    let docked = docked(app, ctx);
    if !docked && !app.piano_roll.open {
        suspend(app, ctx);
        return;
    }
    if app.loaded.is_none() {
        keyboard::cancel(app, ctx);
        return;
    }
    if docked {
        // The pane drew the roll during the main window's pass.
        let outcome = app.piano_roll.pane.take().unwrap_or_default();
        return finish(app, ctx, outcome, true);
    }
    let mut outcome = body::Outcome::default();
    let mut open = true;
    placement(ctx, app.config.piano_roll_maximized)
        .open(&mut open)
        .show(ctx, |ui| {
            if let Some(drawn) = body::show(app, ui) {
                outcome = drawn;
            }
        });
    if !open {
        app.piano_roll.open = false;
    }
    finish(app, ctx, outcome, open);
}

/// Draws the roll as the main window's pane; the rest of the frame's work waits for `show`.
pub(crate) fn pane(app: &mut App, ui: &mut egui::Ui) {
    if crate::view::command_input::preview_active(ui.ctx()) {
        return;
    }
    app.piano_roll.pane = body::show(app, ui);
}

/// The roll lives in a pane, except inside a Views preview, which always shows it floating.
fn docked(app: &App, ctx: &egui::Context) -> bool {
    !crate::view::command_input::preview_active(ctx)
        && crate::view::dock::focus::placed(app, crate::view::dock::ViewKind::PianoRoll)
}

/// A hidden dock window drops the roll's gestures, as a closed roll does.
pub(crate) fn suspend(app: &mut App, ctx: &egui::Context) {
    app.piano_roll.viewport.suspend();
    app.piano_roll.repeat_drag = Default::default();
    keyboard::cancel(app, ctx);
}

fn finish(app: &mut App, ctx: &egui::Context, outcome: body::Outcome, open: bool) {
    let body::Outcome {
        mut released,
        repeat_action,
        mut audition,
        trill_click,
        key,
    } = outcome;
    if body::keyboard_enabled(app) && open {
        keyboard::finish(app, ctx, key);
    } else {
        keyboard::cancel(app, ctx);
    }
    if let Some(tick) = app.piano_roll.viewport.seek.take() {
        crate::view::timeline_domain::click(
            crate::view::timeline_controls::Screen::PianoRoll,
            app,
            app.track,
            tick,
        );
    }
    if trill_click.is_some() {
        audition = None;
        released = None;
    }
    if let Some((slot, track, tick)) = audition {
        if let Some(selected) = app.track_view.split {
            app.track_view.active = if selected[0] == selected[1] {
                slot
            } else {
                selected
                    .iter()
                    .position(|&t| t == track)
                    .unwrap_or(app.track_view.active)
            };
        }
        crate::view::beat_audition::start(app, track, tick);
    }
    if let Some(action) = repeat_action {
        repeat::apply(app, action);
    }
    if let Some((track, on, duration, part, shift)) = released
        && let Err(error) = app.edit_music(|music| {
            cat_smf_editor::edit::trill::ensure_note_editable(music, track, on)?;
            drag::apply(&mut music.tracks[track], on, duration, part, shift)
        })
    {
        app.piano_roll.error = Some(error);
    }
    if let Some((track, event)) = trill_click {
        app.piano_roll.error = crate::view::trill_edit::begin(app, track, event).err();
    }
    chord_timing::show(app, ctx);
}

fn placement(ctx: &egui::Context, maximized: bool) -> egui::Window<'static> {
    if crate::view::command_input::preview_active(ctx) {
        return crate::view::command_input::preview_window(ctx, "Piano roll");
    }
    let window = crate::view::presentation::window(
        ctx,
        crate::view::commands::Screen::Timeline,
        egui::Id::new(if maximized {
            "piano-roll-maximized"
        } else {
            "Piano roll"
        }),
        "Piano roll",
    )
    .collapsible(false);
    if maximized {
        window
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 480.0]).min_height(180.0)
    }
}
