use crate::app::App;
use eframe::egui;

/// Owns input through the dismissal frame, including when another overlay is open.
pub(crate) fn show(app: &mut App, ctx: &egui::Context) -> bool {
    let owned = app.invalid_smf_name.is_some();
    if !owned {
        return false;
    }
    crate::view::commands::route(app, ctx);
    let Some(name) = &app.invalid_smf_name else {
        return owned;
    };
    let mut close = false;
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
