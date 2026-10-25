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
    let patch = &loaded.patches[track];
    let state = &mut app.drum_editor;
    let mut open = state.open;
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
            if state.help {
                egui::Grid::new("drum-help").show(ui, |ui| {
                    super::super::help::key_groups(
                        ui,
                        &[(
                            Color32::LIGHT_BLUE,
                            &[
                                ("h j k l", "移動"),
                                ("f / Space", "Step"),
                                ("Enter", "音色 / Note / Step"),
                                ("PageUp / PageDown", "小節"),
                                ("[ / ]", "A小節"),
                                ("Shift+[ / ]", "B小節"),
                                ("Shift+Space", "停止"),
                                ("Ctrl+Z", "元に戻す"),
                                ("Esc", "閉じる"),
                            ],
                        )],
                    );
                });
                return;
            }
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
                        super::grid::show(ui, &loaded.music, track, patch, state, measure, action);
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
    state.open = open;
    playback::validate(app);
}
