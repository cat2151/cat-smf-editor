use super::{Patch, State};
use eframe::egui;

pub(super) fn handle(
    state: &mut State,
    ctx: &egui::Context,
    patch: &mut Patch,
    choose: &mut Option<u64>,
    error: &mut Option<String>,
) -> bool {
    if state.editing.is_some() {
        return false;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1))
        || ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Text(t) if t == "?"))
        })
    {
        state.help = !state.help;
        return false;
    }
    let mut analysis = false;
    for key in crate::view::comma_menu::pressed(ctx) {
        if state.help {
            if key == egui::Key::Escape {
                state.help = false;
            }
            continue;
        }
        let row = state.row(patch);
        match key {
            egui::Key::Escape => {
                state.open = false;
                break;
            }
            egui::Key::J | egui::Key::ArrowDown => {
                state.selected = patch
                    .layers
                    .get((row + 1).min(patch.layers.len().saturating_sub(1)))
                    .map(|l| l.settings.id);
                state.reveal = true;
            }
            egui::Key::K | egui::Key::ArrowUp => {
                state.selected = patch
                    .layers
                    .get(row.saturating_sub(1))
                    .map(|l| l.settings.id);
                state.reveal = true;
            }
            egui::Key::H | egui::Key::ArrowLeft => state.field = state.field.saturating_sub(1),
            egui::Key::L | egui::Key::ArrowRight => state.field = (state.field + 1).min(2),
            egui::Key::Enter => {
                if state.field == 0 {
                    *choose = state.selected;
                } else {
                    state.begin(patch);
                }
                ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                });
                break;
            }
            egui::Key::O | egui::Key::I | egui::Key::OpenBracket | egui::Key::CloseBracket => {
                let (field, up) = match key {
                    egui::Key::O => (1, true),
                    egui::Key::I => (1, false),
                    egui::Key::CloseBracket => (2, true),
                    _ => (2, false),
                };
                if let Err(e) = super::nudge(patch, row, field, up) {
                    *error = Some(e);
                }
            }
            egui::Key::M | egui::Key::S => {
                let mut candidate = patch.clone();
                if let Some(layer) = candidate.layers.get_mut(row) {
                    let value = if key == egui::Key::M {
                        &mut layer.settings.mute
                    } else {
                        &mut layer.settings.solo
                    };
                    *value = !*value;
                    match candidate.validate() {
                        Ok(()) => *patch = candidate,
                        Err(e) => *error = Some(e),
                    }
                }
            }
            egui::Key::A => {
                if let Err(e) = state.add(patch) {
                    *error = Some(e);
                }
            }
            egui::Key::Delete => {
                if let Some(id) = state.selected {
                    state.remove(patch, id);
                }
            }
            egui::Key::F => analysis = true,
            _ => {}
        }
    }
    analysis
}

pub(super) fn help(state: &mut State, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("レイヤー — ヘルプ");
        if ui.button("×").clicked() {
            state.help = false;
        }
    });
    egui::Grid::new("layer-help").show(ui, |ui| {
        crate::view::help::key_groups(
            ui,
            &[
                (
                    egui::Color32::from_rgb(0x66, 0xd9, 0xef),
                    &[("j k / h l", "レイヤー / 項目")],
                ),
                (
                    egui::Color32::from_rgb(0xf9, 0x26, 0x72),
                    &[
                        ("Enter", "音色選択 / 数値入力"),
                        ("o / i", "Octave +1 / −1"),
                        ("] / [", "音量 +6 / −6 dB"),
                        ("m / s", "mute / solo"),
                        ("a / Delete", "複製 / 削除"),
                        ("f", "Spectrum / Lissajous"),
                        ("Ctrl+Z", "元に戻す"),
                    ],
                ),
            ],
        );
    });
    crate::view::help::close_guide(ui);
}
