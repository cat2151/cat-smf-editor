use super::commands::{apply, Action};
use super::*;
use egui::Color32;
pub(super) fn show(a: &mut App, ctx: &egui::Context) {
    let measure = playback::measure(a);
    let rows = rows(a);
    let patch = a.loaded.as_ref().unwrap().patches[a.track].clone();
    let source = a.loaded.as_ref().unwrap().visible().tracks[a.track].clone();
    let sounding = measure.as_ref().ok().and_then(|m| playback::sounding(a, m));
    let soloable = playback::soloable(a);
    let mut surface = super::super::surface::Surface::new(a, ctx, sounding);
    let mut action = None;
    let mut open = true;
    let window = super::super::window::Kind::Kit;
    window.show(a, ctx, &mut open, |a, ui| {
        let state = &mut a.drum_editor;
        if let Some(input) = &mut state.kit.input {
            ui.label(format!(
                "{} · {}",
                input.note,
                if input.swing { "Swing %" } else { "Velocity" }
            ));
            let response = ui.text_edit_singleline(&mut input.text);
            if std::mem::take(&mut state.kit.focus) {
                response.request_focus();
            }
            ui.horizontal(|ui| {
                if ui.button("Enter · OK").clicked() {
                    action = Some(Action::Confirm);
                }
                if ui.button("Esc · Cancel").clicked() {
                    action = Some(Action::Cancel);
                }
            });
        } else if state.kit.menu {
            ui.horizontal(|ui| {
                if ui.button("Enter · Velocity").clicked() {
                    action = Some(Action::Confirm);
                }
                if ui.button("Esc · Cancel").clicked() {
                    action = Some(Action::Cancel);
                }
            });
        } else {
            let steps = measure.as_ref().map_or(16, |m| m.steps.len());
            ui.horizontal_wrapped(|ui| {
                let mut button = egui::Button::new(patch.label()).truncate();
                if state.field == steps + 2 {
                    button = button.stroke(surface.cursor);
                }
                if ui.add(button).on_hover_text("Kit · Enter").clicked() {
                    state.field = steps + 2;
                    action = Some(Action::Activate);
                }
                if ui.button("◀").on_hover_text("PageUp").clicked() {
                    action = Some(Action::Common(super::super::commands::Op::Page(false)));
                }
                ui.label((state.page + 1).to_string());
                if ui.button("▶").on_hover_text("PageDown").clicked() {
                    action = Some(Action::Common(super::super::commands::Op::Page(true)));
                }
                controls::boundaries(ui, state, |op| action = Some(Action::Common(op)));
                if ui
                    .selectable_label(state.loop_enabled, "Loop")
                    .on_hover_text("R")
                    .clicked()
                {
                    action = Some(Action::Common(super::super::commands::Op::Loop));
                }
                if ui
                    .add_enabled(
                        soloable,
                        egui::SelectableLabel::new(!state.all_tracks, "Solo"),
                    )
                    .on_hover_text("S")
                    .clicked()
                {
                    action = Some(Action::Common(super::super::commands::Op::Scope));
                }
            });
            ui.horizontal(|ui| {
                if ui.button("＋4").on_hover_text("4行追加 · A").clicked() {
                    action = Some(Action::AddRows);
                }
                let editing = state.kit.query_before.is_some();
                if editing {
                    let response = ui.text_edit_singleline(&mut state.kit.query);
                    if std::mem::take(&mut state.kit.focus) {
                        response.request_focus();
                    }
                } else if ui
                    .button(format!("/  {}", state.kit.query))
                    .on_hover_text("Note search · /")
                    .clicked()
                {
                    action = Some(Action::Query);
                }
            });
            if let Ok(m) = &measure {
                let reveal = std::mem::take(&mut state.kit.reveal);
                let row_height = 28.0 + ui.spacing().item_spacing.y;
                let mut scroll_area = egui::ScrollArea::both()
                    .animated(false)
                    .auto_shrink([false, false])
                    .max_height(ui.available_height());
                if reveal && let Some(index) = rows.iter().position(|n| n.slot == state.kit.slot) {
                    scroll_area =
                        scroll_area.vertical_scroll_offset(24.0 + index as f32 * row_height);
                }
                let mut selected_row = None;
                let scroll = scroll_area.show(ui, |ui| {
                    let header_top = ui.cursor().top();
                    ui.add_space(24.0);
                    let mut beat_cells = Vec::new();
                    let mut grid_rect = egui::Rect::NOTHING;
                    for row in &rows {
                        let selected = state.kit.slot == row.slot;
                        ui.horizontal(|ui| {
                            let muted = row
                                .note
                                .is_some_and(|n| patch.kit.as_ref().unwrap().muted.contains(&n));
                            let response = ui
                                .add_enabled(
                                    row.note.is_some(),
                                    egui::Button::new("M").selected(muted),
                                )
                                .on_hover_text("Mute · M");
                            #[cfg(test)]
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new(("kit-mute", row.slot)), response.rect)
                            });
                            if response.clicked() {
                                state.kit.slot = row.slot;
                                state.kit.note = row.note;
                                action = Some(Action::Mute);
                            }
                            let mut header = egui::Button::new(super::super::surface::lane_text(
                                if row.name.is_empty() {
                                    "＋"
                                } else {
                                    &row.name
                                },
                                muted,
                            ))
                            .truncate();
                            if selected && state.field == 0 {
                                header = header.stroke(surface.cursor);
                            }
                            let response = ui
                                .add_sized([106.0, 28.0], header)
                                .on_hover_text(format!("{} · N", row.name));
                            #[cfg(test)]
                            ctx.data_mut(|d| {
                                d.insert_temp(egui::Id::new(("kit-name", row.slot)), response.rect)
                            });
                            surface.selection(ui, response.rect, selected && state.field == 0);
                            if selected {
                                selected_row = Some(response.rect);
                            }
                            if reveal && selected && state.field == 0 {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                state.kit.slot = row.slot;
                                state.kit.note = row.note;
                                state.field = 0;
                                action = Some(Action::Pick);
                            }
                            controls::swing_controls(
                                ui,
                                state,
                                row,
                                patch.kit.as_ref().unwrap(),
                                steps,
                                reveal,
                                surface.cursor,
                                &mut action,
                            );
                            let mut velocity = egui::Button::new(row.velocity.to_string());
                            if selected && state.field == 1 {
                                velocity = velocity.stroke(surface.cursor);
                            }
                            let response = ui
                                .add_enabled(
                                    row.note.is_some(),
                                    velocity.min_size(egui::vec2(38.0, 28.0)),
                                )
                                .on_hover_text("Velocity · Enter / Wheel");
                            #[cfg(test)]
                            ctx.data_mut(|d| {
                                d.insert_temp(
                                    egui::Id::new(("kit-velocity", row.slot)),
                                    response.rect,
                                )
                            });
                            if let Some(up) = controls::wheel(ui, &response)
                                && let Some(note) = row.note
                            {
                                action = Some(Action::Velocity(
                                    note,
                                    super::super::values::Field::Velocity.stepped(row.velocity, up),
                                ));
                            }
                            if reveal && selected && state.field == 1 {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                state.kit.slot = row.slot;
                                state.kit.note = row.note;
                                state.field = 1;
                                action = Some(Action::Activate);
                            }
                            for step in 0..steps {
                                if step % 4 == 0 {
                                    ui.add_space(3.0);
                                }
                                let hit = row.note.is_some_and(|note| {
                                    !drum_kit::hits(&source, m, note, step).is_empty()
                                });
                                let mut button = egui::Button::new("")
                                    .fill(super::super::surface::step_fill(ui, hit, muted));
                                if selected && state.field == step + 2 {
                                    button = button.stroke(surface.cursor);
                                }
                                let response = ui.add_enabled(
                                    row.note.is_some(),
                                    button.min_size(egui::vec2(22.0, 28.0)),
                                );
                                surface.selection(
                                    ui,
                                    response.rect,
                                    selected && state.field == step + 2,
                                );
                                surface.cell(response.rect, step);
                                grid_rect = grid_rect.union(response.rect);
                                if row.slot == rows[0].slot {
                                    beat_cells.push((step, response.rect));
                                }
                                #[cfg(test)]
                                ctx.data_mut(|d| {
                                    d.insert_temp(
                                        egui::Id::new(("kit-cell", row.note.unwrap_or(255), step)),
                                        response.rect,
                                    )
                                });
                                if reveal && selected && state.field == step + 2 {
                                    response.scroll_to_me(None);
                                }
                                if response.clicked() || response.secondary_clicked() {
                                    state.kit.slot = row.slot;
                                    state.kit.note = row.note;
                                    state.field = step + 2;
                                    action = Some(if response.secondary_clicked() {
                                        Action::Menu
                                    } else {
                                        Action::Activate
                                    });
                                }
                            }
                        });
                    }
                    grid_rect.min.y = header_top;
                    for (step, cell) in beat_cells {
                        let header = egui::Rect::from_min_size(
                            egui::pos2(cell.left(), header_top),
                            egui::vec2(cell.width(), 24.0),
                        );
                        super::super::surface::column(
                            ui,
                            step,
                            header,
                            cell.width(),
                            cell.left() - 3.5,
                            grid_rect,
                        );
                    }
                    surface.finish(ui);
                });
                if reveal
                    && selected_row.is_some_and(|row| {
                        row.top() < scroll.inner_rect.top()
                            || row.bottom() > scroll.inner_rect.bottom()
                    })
                {
                    // Initial sizing can clamp the offset before rows reach their final height.
                    state.kit.reveal = true;
                    ctx.request_repaint();
                }
                #[cfg(test)]
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("kit-clip"), scroll.inner_rect));
                #[cfg(not(test))]
                let _ = scroll;
            } else if let Err(error) = &measure {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        }
        if let Some(error) = &a.drum_editor.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    });
    if !open {
        action = Some(Action::Cancel);
    }
    if let Some(action) = action {
        apply(a, action);
    }
    if a.drum_editor.kit.preview.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(35));
    }
}
