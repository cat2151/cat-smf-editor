//! The same note-onset labels on the piano roll and the tempo curve's note pane.
use super::Layout;
use eframe::egui::{self, Color32};

pub(crate) fn draw(
    painter: &egui::Painter,
    layout: &Layout,
    note_rect: impl Fn(usize) -> egui::Rect,
) {
    for (i, text) in layout.lyrics.iter().enumerate() {
        let Some(text) = text else { continue };
        let rect = note_rect(i);
        let galley = painter.layout_no_wrap(
            text.clone(),
            egui::FontId::proportional(rect.height().clamp(14.0, 18.0)),
            Color32::BLACK,
        );
        let label = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0 - 1.0),
            galley.size() + egui::vec2(4.0, 2.0),
        );
        if painter.clip_rect().intersects(label) {
            painter.rect_filled(label, 0.0, Color32::WHITE);
            painter.galley(label.min + egui::vec2(2.0, 1.0), galley, Color32::BLACK);
        }
    }
}
