//! Project out-of-range tempos onto the BPM panel edges without changing their values.
use super::*;

impl Roll {
    pub(super) fn pinned(&self, mut pos: egui::Pos2) -> egui::Pos2 {
        pos.y = pos.y.clamp(self.rect.top(), self.rect.bottom());
        pos
    }

    /// A chevron points toward the true position, entirely inside the panel.
    pub(super) fn edge_marker(&self, painter: &egui::Painter, pos: egui::Pos2, color: Color32) {
        let direction = if pos.y < self.rect.top() {
            1.0
        } else if pos.y > self.rect.bottom() {
            -1.0
        } else {
            return;
        };
        let tip = self.pinned(pos) + egui::vec2(0.0, direction * 2.0);
        painter.add(egui::Shape::line(
            vec![
                tip + egui::vec2(-4.0, direction * 5.0),
                tip,
                tip + egui::vec2(4.0, direction * 5.0),
            ],
            egui::Stroke::new(2.0_f32, color),
        ));
    }
}
