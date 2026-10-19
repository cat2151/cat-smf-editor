use super::{Patch, State};
use eframe::egui;

pub(super) fn show(
    state: &mut State,
    ctx: &egui::Context,
    patch: &mut Patch,
    choose: &mut Option<u64>,
    error: &mut Option<String>,
    prior_error: Option<&str>,
) {
    let mut remove = None;
    let mut wheel = None;
    let mut cancel = false;
    if state.editing.is_some() {
        cancel = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    }
    egui::Modal::new(egui::Id::new("instrument-layers")).show(ctx, |ui| {
        if state.help {
            super::keyboard::help(state, ui);
            return;
        }
        ui.scope(|ui| {
            ui.set_max_height((ctx.screen_rect().height() - 40.0).max(22.0));
            ui.set_width((ctx.screen_rect().width() - 40.0).clamp(300.0, 600.0));
            let patch_width = (ui.available_width() - 260.0).max(40.0);
            ui.horizontal(|ui| {
                ui.heading("Instrument layers");
                if ui.button("+").clicked()
                    && let Err(e) = state.add(patch)
                {
                    *error = Some(e);
                }
                if ui.button("×").clicked() {
                    state.open = false;
                }
            });
            egui::ScrollArea::vertical()
                .max_height((ctx.screen_rect().height() - 100.0).max(22.0))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    egui::Grid::new("layer-rows").striped(true).show(ui, |ui| {
                        for heading in ["Patch", "Octave", "dB", "Mute", "Solo", ""] {
                            ui.label(heading);
                        }
                        ui.end_row();
                        for layer in &mut patch.layers {
                            let id = layer.settings.id;
                            let selected = state.selected == Some(id);
                            let label = layer.label();
                            let response = ui
                                .push_id((id, "patch"), |ui| {
                                    ui.add_sized(
                                        [patch_width, 22.0],
                                        egui::Button::new(label)
                                            .truncate()
                                            .selected(selected && state.field == 0),
                                    )
                                    .on_hover_text(label)
                                })
                                .inner;
                            if selected && state.reveal {
                                response.scroll_to_me(Some(egui::Align::Center));
                                state.reveal = false;
                            }
                            if response.clicked() {
                                state.selected = Some(id);
                                state.field = 0;
                                state.editing = None;
                                *choose = Some(id);
                            }
                            for field in [1, 2] {
                                let active = state.selected == Some(id) && state.field == field;
                                let response = ui
                                    .push_id((id, field), |ui| {
                                        if active && let Some(edit) = &mut state.editing {
                                            let mut output =
                                                egui::TextEdit::singleline(&mut edit.text)
                                                    .desired_width(70.0)
                                                    .show(ui);
                                            if edit.focus {
                                                output.response.request_focus();
                                                output.state.cursor.set_char_range(Some(
                                                    egui::text::CCursorRange::two(
                                                        egui::text::CCursor::new(0),
                                                        egui::text::CCursor::new(
                                                            edit.text.chars().count(),
                                                        ),
                                                    ),
                                                ));
                                                output.state.store(ui.ctx(), output.response.id);
                                                edit.focus = false;
                                            }
                                            if !cancel {
                                                cancel |= output.response.lost_focus()
                                                    && !crate::view::palette::suspended(ctx);
                                            }
                                            output.response
                                        } else {
                                            let text = if field == 1 {
                                                layer.settings.octave.to_string()
                                            } else {
                                                layer.settings.volume_db.to_string()
                                            };
                                            ui.add_sized(
                                                [70.0, 22.0],
                                                egui::Button::new(text).selected(active),
                                            )
                                        }
                                    })
                                    .inner;
                                if state.editing.is_none() && response.hovered() {
                                    let steps: i32 = ui.input_mut(|i| {
                                        i.smooth_scroll_delta = egui::Vec2::ZERO;
                                        i.raw_scroll_delta = egui::Vec2::ZERO;
                                        i.events
                                            .iter()
                                            .map(|e| match e {
                                                egui::Event::MouseWheel { delta, .. } => {
                                                    (delta.y > 0.0) as i32 - (delta.y < 0.0) as i32
                                                }
                                                _ => 0,
                                            })
                                            .sum()
                                    });
                                    if steps != 0 {
                                        wheel = Some((id, field, steps));
                                    }
                                }
                                if response.clicked() && !(active && state.editing.is_some()) {
                                    state.selected = Some(id);
                                    state.field = field;
                                    state.editing = Some(super::Editing {
                                        text: if field == 1 {
                                            layer.settings.octave.to_string()
                                        } else {
                                            layer.settings.volume_db.to_string()
                                        },
                                        focus: true,
                                    });
                                }
                            }
                            for (field, value, label) in [
                                (3, &mut layer.settings.mute, "M"),
                                (4, &mut layer.settings.solo, "S"),
                            ] {
                                if ui
                                    .push_id((id, field), |ui| {
                                        ui.add(egui::Button::new(label).selected(*value))
                                    })
                                    .inner
                                    .clicked()
                                {
                                    state.selected = Some(id);
                                    state.editing = None;
                                    *value = !*value;
                                }
                            }
                            if ui
                                .push_id((id, "remove"), |ui| ui.button("−"))
                                .inner
                                .clicked()
                            {
                                remove = Some(id);
                            }
                            ui.end_row();
                        }
                    });
                });
            if let Some(e) = error.as_deref().or(prior_error) {
                ui.colored_label(egui::Color32::LIGHT_RED, e);
            }
        });
    });
    if cancel {
        state.editing = None;
    }
    if let Some((id, field, steps)) = wheel {
        state.selected = Some(id);
        state.field = field;
        let row = state.row(patch);
        for _ in 0..steps.unsigned_abs() {
            if let Err(e) = super::nudge(patch, row, field, steps > 0) {
                *error = Some(e);
                break;
            }
        }
    }
    if let Some(id) = remove {
        state.remove(patch, id);
    }
}
