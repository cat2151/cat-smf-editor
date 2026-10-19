use super::Job;
use eframe::egui::{self, Color32, RichText};

pub(super) fn show(job: &Job, ctx: &egui::Context) {
    // Plain UI paints on its first frame, including when the recent-files modal closes.
    let screen = ctx.screen_rect().shrink(16.0);
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("song-opening"));
    let mut ui = egui::Ui::new(
        ctx.clone(),
        layer.id,
        egui::UiBuilder::new()
            .layer_id(layer)
            .max_rect(egui::Rect::from_min_size(
                screen.min,
                egui::vec2(screen.width().min(500.0), screen.height()),
            )),
    );
    egui::Frame::popup(&ctx.style()).show(&mut ui, |ui| {
        ui.set_max_width((screen.width() - 24.0).clamp(1.0, 476.0));
        ui.strong(&job.name);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(job.title);
        });
        egui::ScrollArea::vertical()
            .max_height((screen.height() - 100.0).max(30.0))
            .show(ui, |ui| {
                egui::Grid::new("song-opening-tracks")
                    .max_col_width(((screen.width() - 64.0) / 2.0).clamp(1.0, 226.0))
                    .spacing([20.0, 6.0])
                    .show(ui, |ui| {
                        for (index, (name, status)) in job.rows.iter().enumerate() {
                            let active = job.current == Some(index);
                            let color = if active {
                                super::super::ACTIVE_PANE
                            } else if *status == "Ready" {
                                Color32::from_rgb(0xa6, 0xe2, 0x2e)
                            } else {
                                ui.visuals().text_color()
                            };
                            let response = ui.label(RichText::new(name).color(color));
                            ui.label(RichText::new(*status).color(color));
                            if active {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            ui.end_row();
                        }
                    });
            });
    });
}
