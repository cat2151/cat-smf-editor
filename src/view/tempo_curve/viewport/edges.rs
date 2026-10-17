//! Adjust one end of the visible time range, keeping the opposite end fixed.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Drag {
    left: f32,
    right: f32,
    origin: f32,
    scale: f32,
}

impl Viewport {
    pub(super) fn adjust_edge(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        scale: f32,
        response: &egui::Response,
    ) {
        let secondary = egui::PointerButton::Secondary;
        if response.drag_started_by(secondary) {
            self.edge_drag = Some(Drag {
                left: self.scroll / scale,
                right: (self.scroll + self.width) / scale,
                origin: rect.left(),
                scale,
            });
        }
        let clicked = response.secondary_clicked();
        let Some(drag) = self.edge_drag.or_else(|| {
            clicked.then_some(Drag {
                left: self.scroll / scale,
                right: (self.scroll + self.width) / scale,
                origin: rect.left(),
                scale,
            })
        }) else {
            return;
        };
        if let Some(pos) = response.interact_pointer_pos() {
            let tick = ((pos.x - drag.origin) / drag.scale).max(0.0);
            let (left, right) = if (tick - drag.left).abs() <= (tick - drag.right).abs() {
                (tick.min(drag.right - 1.0).max(0.0), drag.right)
            } else {
                (drag.left, tick.max(drag.left + 1.0))
            };
            let selected = egui::Rect::from_x_y_ranges(
                drag.origin + left * drag.scale..=drag.origin + right * drag.scale,
                rect.y_range(),
            );
            ui.painter()
                .with_clip_rect(ui.clip_rect().intersect(rect))
                .rect_filled(selected, 0.0, TEMPO.gamma_multiply(0.25));
            if clicked || response.drag_stopped_by(secondary) {
                self.zoom_time(
                    self.zoom * self.width * 0.9 / ((right - left) * scale),
                    (left + right) * scale / 2.0 - self.scroll,
                );
            }
        }
        if response.drag_stopped_by(secondary) || !ui.input(|i| i.pointer.button_down(secondary)) {
            self.edge_drag = None;
        }
    }
}
