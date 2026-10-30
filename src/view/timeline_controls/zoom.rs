//! One horizontal zoom gesture implementation for every registered screen.
use super::*;

#[derive(Default)]
pub(crate) struct Zoom {
    start: Option<f32>,
    range: Option<(f32, f32)>,
}

impl Zoom {
    pub fn range(&self) -> Option<(f32, f32)> {
        self.range
    }

    /// Returns a factor and an anchor in the actual content viewport.
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        zone: egui::Rect,
        view: (Screen, egui::Rect),
        enabled: bool,
        tint: egui::Color32,
        response: &egui::Response,
    ) -> Option<(f32, f32)> {
        let (screen, content) = view;
        assert!(Screen::ALL.contains(&screen) && screen.has_bars());
        self.range = None;
        if !enabled || !ui.is_enabled() {
            self.start = None;
            return None;
        }
        let local = |p: egui::Pos2| (p.x - content.left()).clamp(0.0, content.width().max(0.0));
        let completed = super::super::timeline_drag::completed(ui, zone).or_else(|| {
            super::super::timeline_drag::completed_by(ui, zone, egui::PointerButton::Secondary)
        });
        if response.drag_started() || completed.is_some() {
            self.start = completed
                .map(|p| p.0)
                .or_else(|| ui.input(|i| i.pointer.press_origin()))
                .map(local);
        }
        let mut action = None;
        if let Some((start, pos)) = self.start.zip(
            completed
                .map(|p| p.1)
                .or_else(|| response.interact_pointer_pos()),
        ) {
            let end = local(pos);
            let selected = zone
                .with_min_x(content.left() + start.min(end))
                .with_max_x(content.left() + start.max(end));
            ui.painter()
                .with_clip_rect(ui.clip_rect().intersect(zone))
                .rect_filled(selected, 0.0, tint.gamma_multiply(0.5));
            // The bar is drawn before the content, so the content's own layer would cover it.
            ui.ctx()
                .layer_painter(egui::LayerId::new(
                    ui.layer_id().order,
                    ui.id().with("zoom-range"),
                ))
                .with_clip_rect(ui.clip_rect().intersect(content))
                .rect_filled(
                    selected
                        .with_min_y(content.top())
                        .with_max_y(content.bottom()),
                    0.0,
                    tint.gamma_multiply(0.2),
                );
            let released = response.drag_stopped() || completed.is_some();
            if !released && (start - end).abs() > 2.0 {
                self.range = Some((start, end));
            }
            if released && (start - end).abs() > 2.0 {
                action = Some((
                    content.width() * timeline_zoom::FIT / (start - end).abs(),
                    (start + end) / 2.0,
                ));
            }
        }
        if action.is_none()
            && (response.clicked() || response.secondary_clicked())
            && let Some(pos) = response.interact_pointer_pos()
        {
            action = Some((
                if response.clicked() {
                    timeline_zoom::CLICK
                } else {
                    1.0 / timeline_zoom::CLICK
                },
                local(pos),
            ));
        }
        if response.hovered() {
            let delta = ui.input_mut(|i| {
                let d = if i.smooth_scroll_delta.y != 0.0 {
                    i.smooth_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.x
                };
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                i.raw_scroll_delta = egui::Vec2::ZERO;
                d
            });
            if delta != 0.0 && self.start.is_none() {
                action = Some((timeline_zoom::wheel(delta), content.width() / 2.0));
            }
        }
        if !ui.input(|i| i.pointer.any_down()) {
            self.start = None;
        }
        action
    }
}
