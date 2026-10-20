//! First-frame progress, matching the song loading popup at the top left.
use crate::app::App;
use eframe::egui;

pub(crate) fn show(app: &App, ctx: &egui::Context) {
    if super::super::opening::pending(app) {
        return;
    }
    let Some(title) = crate::play::sung::prerender::progress(app) else {
        return;
    };
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("voicevox-progress"));
    let mut ui = egui::Ui::new(
        ctx.clone(),
        layer.id,
        egui::UiBuilder::new()
            .layer_id(layer)
            .max_rect(ctx.screen_rect().shrink(16.0)),
    );
    egui::Frame::popup(&ctx.style()).show(&mut ui, |ui| {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(title);
        });
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(50));
}
