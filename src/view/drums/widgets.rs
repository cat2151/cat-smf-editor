use super::commands::Op;
use super::*;
use egui::Color32;

pub(super) fn show(app: &mut App, ctx: &egui::Context, action: &mut Option<Op>) {
    if app.drum_editor.confirm_create {
        let response = egui::Modal::new(egui::Id::new("drum-create")).show(ctx, |ui| {
            ui.label("Create a new drum track?");
            ui.horizontal(|ui| {
                if ui.button("Yes · Enter").clicked() {
                    *action = Some(Op::Create);
                }
                if ui.button("No · Esc").clicked() {
                    *action = Some(Op::Cancel);
                }
            });
            if let Some(error) = &app.drum_editor.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        });
        if response.should_close() && action.is_none() {
            *action = Some(Op::Cancel);
        }
        return;
    }
    let measure = playback::measure(app);
    let Some(loaded) = &app.loaded else {
        return;
    };
    let track = app.track;
    let beacon = crate::view::patterns::beacon::focused(ctx)
        && crate::view::patterns::mode::editing(app, track)
            .is_some_and(|c| crate::view::patterns::beacon::empty(&loaded.music, c.pattern));
    let patch = &loaded.patches[track];
    let state = &mut app.drum_editor;
    let mut open = true;
    egui::Window::new(format!("Drum sequencer · {}", loaded.track_titles[track]))
        .id(egui::Id::new("drum-sequencer"))
        .open(&mut open)
        .collapsible(false)
        .resizable([true, false])
        .scroll([false, false])
        .min_width((ctx.screen_rect().width() - 32.0).clamp(1.0, 320.0))
        .max_width((ctx.screen_rect().width() - 32.0).max(1.0))
        .default_width((ctx.screen_rect().width() - 32.0).clamp(1.0, 1080.0))
        .show(ctx, |ui| {
            if let Some(Input::Note { text, focus }) = &mut state.input {
                ui.label(format!("{} · Note", drums::NAMES[state.row]));
                let response = ui.text_edit_singleline(text);
                if *focus {
                    response.request_focus();
                    *focus = false;
                }
                ui.horizontal(|ui| {
                    if ui.button("Enter · OK").clicked() {
                        *action = Some(Op::Confirm);
                    }
                    if ui.button("Esc · Cancel").clicked() {
                        *action = Some(Op::Cancel);
                    }
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("◀").clicked() {
                        *action = Some(Op::Page(false));
                    }
                    ui.label(format!("meas. {}", state.page + 1));
                    if ui.button("▶").clicked() {
                        *action = Some(Op::Page(true));
                    }
                    ui.separator();
                    for (end, label, number) in [(false, "A", state.first), (true, "B", state.last)]
                    {
                        ui.label(label);
                        let r = ui.button(format!("meas. {number}"));
                        if r.hovered() {
                            let delta = ui.input(|i| i.raw_scroll_delta.y);
                            if delta != 0.0 {
                                ui.input_mut(|i| {
                                    i.raw_scroll_delta = egui::Vec2::ZERO;
                                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                                });
                                *action = Some(Op::Boundary(end, if delta > 0.0 { 1 } else { -1 }));
                            }
                        }
                    }
                    if ui
                        .selectable_label(state.loop_enabled, "Loop · r")
                        .clicked()
                    {
                        *action = Some(Op::Loop);
                    }
                    if ui
                        .button(if state.all_tracks {
                            "All · s"
                        } else {
                            "Solo · s"
                        })
                        .clicked()
                    {
                        *action = Some(Op::Scope);
                    }
                });
                match &measure {
                    Ok(measure) => {
                        super::grid::show(
                            ui,
                            loaded.visible(),
                            track,
                            patch,
                            state,
                            measure,
                            beacon,
                            action,
                        );
                    }
                    Err(e) => {
                        ui.colored_label(Color32::LIGHT_RED, e);
                    }
                }
            }
            if let Some(error) = &state.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        });
    if !open {
        *action = Some(Op::Close);
    }
}
