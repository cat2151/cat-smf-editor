//! Keep conversion and plugin initialization out of the input's paint frame.
use super::{Action, State};
use eframe::egui;

pub(super) fn show(ctx: &egui::Context, state: &mut State, app: &crate::app::App) -> Action {
    let mut action = state.command.take().unwrap_or(Action::None);
    let name = crate::view::patterns::presentation::name(app, app.track);
    let modal = egui::Modal::new(egui::Id::new("mml-editor"))
        .frame(crate::view::patterns::presentation::modal_frame(
            app, ctx, app.track,
        ))
        .show(ctx, |ui| {
            ui.set_width(520.0);
            if let Some(name) = &name {
                crate::view::patterns::presentation::label(ui, name);
            }
            ui.heading("MML / Chord progression");
            let output = egui::TextEdit::multiline(&mut state.buffer)
                .id_salt("mml_editor_text")
                .font(egui::TextStyle::Monospace)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .show(ui);
            if !crate::view::palette::suspended(ctx)
                && let Some(range) = output.cursor_range
            {
                let cursor = range.primary.ccursor.index;
                let changed = output.response.changed();
                let previous = state.preview_cursor;
                if changed || previous != Some(cursor) && !state.buffer.is_empty() {
                    // Layout retries must not consume work from this same frame.
                    let changed =
                        changed || state.pending_preview.is_some_and(|(_, _, changed)| changed);
                    state.pending_preview = Some((ctx.cumulative_pass_nr(), cursor, changed));
                    ctx.request_repaint();
                }
                state.preview_cursor = Some(cursor);
            }
            if state.focus {
                output.response.request_focus();
                state.focus = false;
            }
            // Reserve the same row while idle so the centered modal never resizes
            // when preparation starts or finishes.
            let mut row = egui::UiBuilder::new();
            if state.pending_preview.is_none() {
                row = row.invisible();
            }
            ui.scope_builder(row, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Preparing playback, please wait...");
                });
            });
            if let Some(error) = &state.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                if ui.button("Enter: Confirm").clicked() {
                    action = Action::Confirm;
                }
                if ui.button("Esc: Cancel").clicked() {
                    action = Action::Cancel;
                }
            });
        });
    if modal.should_close() && !crate::view::palette::suspended(ctx) {
        action = Action::Cancel;
    }
    action
}
