//! Select a range on a zoom zone and fit it to 90% on release.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Drag {
    edge: usize,
    start: f32,
    panel: egui::Rect,
}

impl Viewport {
    pub(super) fn drag_zoom(
        &mut self,
        ui: &egui::Ui,
        zone: egui::Rect,
        base: Axis,
        enabled: bool,
        edge: usize,
        response: &egui::Response,
    ) {
        if !enabled {
            self.zoom_drag = None;
            return;
        }
        let primary = egui::PointerButton::Primary;
        if response.drag_started_by(primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            && let Some(panel) = self.tempo_rect
        {
            self.zoom_drag = Some(Drag {
                edge,
                start: if edge < 2 { origin.x } else { origin.y },
                panel,
            });
        }
        let Some(drag) = self.zoom_drag.filter(|d| d.edge == edge) else {
            return;
        };
        if let Some(pos) = response.interact_pointer_pos() {
            let (origin, length, end) = if edge < 2 {
                (drag.panel.left(), self.width, pos.x)
            } else {
                (drag.panel.top(), drag.panel.height(), pos.y)
            };
            let length = length.max(1.0);
            let a = (drag.start - origin).clamp(0.0, length);
            let b = (end - origin).clamp(0.0, length);
            let mut selected = zone;
            let axis = usize::from(edge >= 2);
            selected.min[axis] = origin + a.min(b);
            selected.max[axis] = origin + a.max(b);
            ui.painter()
                .with_clip_rect(ui.clip_rect().intersect(zone))
                .rect_filled(selected, 0.0, TEMPO.gamma_multiply(0.5));
            if response.drag_stopped_by(primary) && (a - b).abs() > 2.0 {
                let factor = length * timeline_zoom::FIT / (a - b).abs();
                let center = (a + b) / 2.0;
                if edge < 2 {
                    self.zoom_time(self.zoom * factor, center);
                } else {
                    self.zoom_bpm(base, f64::from(factor), center / length);
                }
            }
        }
        if response.drag_stopped_by(primary) || !ui.input(|i| i.pointer.button_down(primary)) {
            self.zoom_drag = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variation_vertical_bar_preserves_horizontal_range_selection() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0));
        let mut view = Viewport {
            limit_to_fit: true,
            zoom_drag: Some(Drag {
                edge: 0,
                start: 30.0,
                panel: rect,
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response =
                    ui.allocate_response(egui::vec2(20.0, 200.0), egui::Sense::click_and_drag());
                view.drag_zoom(
                    ui,
                    rect,
                    Axis {
                        low: 0.0,
                        high: 1.0,
                    },
                    true,
                    2,
                    &response,
                );
            });
        });
        assert_eq!(view.zoom_drag.map(|drag| drag.edge), Some(0));
    }
}
