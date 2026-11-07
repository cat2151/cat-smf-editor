use super::commands::Op;
use super::*;
use egui::Color32;

const LOOP_HOVER: &str = "A-B間をLoop  R";
const SOLO_HOVER: &str = "Loopでこのtrackだけを鳴らす  S";

pub(super) fn show(app: &mut App, ctx: &egui::Context, action: &mut Option<Op>) {
    if app.drum_editor.confirm_create {
        let response =
            crate::view::presentation::modal(ctx, egui::Id::new("drum-create")).show(ctx, |ui| {
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
    let soloable = playback::soloable(app);
    let sounding_span = crate::frame_trace::span("contents/drums/sounding");
    let sounding = measure
        .as_ref()
        .ok()
        .and_then(|m| playback::sounding(app, m));
    drop(sounding_span);
    let Some(_) = &app.loaded else {
        return;
    };
    let track = app.track;
    let surface = super::surface::Surface::new(app, ctx, sounding);
    let window = super::window::Kind::Drums;
    let mut open = true;
    window.show(app, ctx, &mut open, |app, ui| {
        let loaded = app.loaded.as_ref().unwrap();
        let patch = &loaded.patches[track];
        let state = &mut app.drum_editor;
        if let Some(input) = &mut state.input {
            let (label, text, focus) = match input {
                Input::Note { text, focus } => ("Note", text, focus),
                Input::Velocity { text, focus } => ("Velocity", text, focus),
                Input::Level { text, focus } => ("Level · dB", text, focus),
                Input::Swing { text, focus } => ("Swing %", text, focus),
            };
            ui.label(format!("{} · {label}", drums::NAMES[state.row]));
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
                for (end, label, number) in [(false, "A", state.first), (true, "B", state.last)] {
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
                    .selectable_label(state.loop_enabled, "Loop")
                    .on_hover_text(LOOP_HOVER)
                    .clicked()
                {
                    *action = Some(Op::Loop);
                }
                if ui
                    .add_enabled(
                        soloable,
                        egui::SelectableLabel::new(!state.all_tracks, "Solo"),
                    )
                    .on_hover_text(SOLO_HOVER)
                    .on_disabled_hover_text(SOLO_HOVER)
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
                        surface,
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
