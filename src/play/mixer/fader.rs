//! A dB fader with a scale beside its own rail, including after horizontal scrolling.
use crate::play::track_volume::{MAX_DB, MIN_DB, STEP_DB};
use eframe::egui;

const HEIGHT: f32 = 252.0;

pub(super) fn show(ui: &mut egui::Ui, db: &mut i32) -> egui::Response {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().slider_width = HEIGHT;
        let response = ui.add(
            egui::Slider::new(db, MIN_DB..=MAX_DB)
                .vertical()
                .handle_shape(egui::style::HandleShape::Circle)
                .trailing_fill(true)
                .step_by(STEP_DB as f64)
                .show_value(false),
        );
        let (scale, _) = ui.allocate_exact_size(egui::vec2(58.0, HEIGHT), egui::Sense::hover());
        // egui 0.31's circular handle moves within the response rect inset by
        // its radius (short side / 2.5). Match that travel, rather than the rail ends.
        let travel = response.rect.shrink(response.rect.width() / 2.5);
        for value in (MIN_DB..=MAX_DB).step_by(STEP_DB as usize) {
            let y = egui::lerp(
                travel.bottom()..=travel.top(),
                (value - MIN_DB) as f32 / (MAX_DB - MIN_DB) as f32,
            );
            let major = value % 6 == 0;
            let color = if value == 0 {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().text_color()
            };
            ui.painter().line_segment(
                [
                    egui::pos2(scale.left(), y),
                    egui::pos2(scale.left() + if major { 10.0 } else { 5.0 }, y),
                ],
                egui::Stroke::new(if value == 0 { 2.0_f32 } else { 1.0_f32 }, color),
            );
            if major {
                ui.painter().text(
                    egui::pos2(scale.left() + 14.0, y),
                    egui::Align2::LEFT_CENTER,
                    format!("{value:+} dB"),
                    egui::FontId::proportional(12.0),
                    color,
                );
            }
        }
        response
    })
    .inner
}
