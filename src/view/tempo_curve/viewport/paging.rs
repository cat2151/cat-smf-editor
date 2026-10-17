//! Half a viewport per click along the scrollbar axis.
use super::*;

pub(super) const BUTTON: f32 = 16.0;

impl Viewport {
    pub(crate) fn horizontal_pages(&mut self, ui: &mut egui::Ui, bar: egui::Rect) {
        self.horizontal_bar = Some(bar);
        if let Some(direction) = buttons(ui, bar, false) {
            self.pending_scroll = Some((self.scroll + direction * self.width * 0.5).max(0.0));
        }
    }
}

/// Returns -1 for the start of the bar and +1 for its end.
pub(super) fn buttons(ui: &mut egui::Ui, bar: egui::Rect, vertical: bool) -> Option<f32> {
    let mut clicked = None;
    for end in [false, true] {
        let mut rect = bar;
        let axis = usize::from(vertical);
        if end {
            rect.min[axis] = (rect.max[axis] - BUTTON).max(rect.min[axis]);
        } else {
            rect.max[axis] = (rect.min[axis] + BUTTON).min(rect.max[axis]);
        }
        let response = ui.interact(
            rect,
            ui.id().with(("tempo-page", vertical, end)),
            egui::Sense::click(),
        );
        let visuals = ui.style().interact(&response);
        ui.painter()
            .rect_filled(rect.shrink(1.0), 2.0, visuals.bg_fill);
        let direction = if end { 1.0 } else { -1.0 };
        let along = if vertical {
            egui::vec2(0.0, direction)
        } else {
            egui::vec2(direction, 0.0)
        };
        let across = egui::vec2(-along.y, along.x);
        let center = rect.center();
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                center + along * 4.0,
                center - along * 3.0 + across * 3.0,
                center - along * 3.0 - across * 3.0,
            ],
            visuals.fg_stroke.color,
            egui::Stroke::NONE,
        ));
        if response.clicked() {
            clicked = Some(direction);
        }
    }
    clicked
}
