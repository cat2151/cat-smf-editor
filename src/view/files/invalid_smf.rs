use crate::app::App;
use eframe::egui::{self, Key, Modifiers};

/// Owns input through the dismissal frame, including when another overlay is open.
pub(crate) fn show(app: &mut App, ctx: &egui::Context) -> bool {
    let Some(name) = &app.invalid_smf_name else {
        return false;
    };
    let mut close = ctx.input_mut(|i| {
        let enter = i.consume_key(Modifiers::NONE, Key::Enter);
        let escape = i.consume_key(Modifiers::NONE, Key::Escape);
        enter || escape
    });
    egui::Modal::new(egui::Id::new("invalid-smf")).show(ctx, |ui| {
        ui.set_max_width(480.0);
        ui.strong(name);
        ui.label("このファイルはスタンダードMIDIファイルとして認識できませんでした。ファイルを見直してください");
        close |= ui.button("閉じる").clicked();
    });
    super::super::comma_menu::swallow(ctx);
    if close {
        app.invalid_smf_name = None;
        app.load_error = None;
    }
    true
}
