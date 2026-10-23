use eframe::egui::{self, Color32};

pub(crate) const HEIGHT: f32 = 24.0;
pub(crate) const PLAYHEAD: Color32 = Color32::from_rgb(0xfd, 0x97, 0x1f);
pub(crate) const RANGE: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);
const RESTART_INTERVAL: f64 = 0.25;

/// Conversion between ticks and x measured from the left edge of the row.
pub(crate) trait TickAxis {
    fn x(&self, tick: u64) -> f32;
    fn tick(&self, x: f32) -> u64;
}

/// Evenly spaced ticks, in pixels per tick.
impl TickAxis for f32 {
    fn x(&self, tick: u64) -> f32 {
        tick as f32 * self
    }

    fn tick(&self, x: f32) -> u64 {
        (x / self).round().max(0.0) as u64
    }
}

#[derive(Default)]
pub(crate) struct State {
    left: Option<u64>,
    right: Option<RightDrag>,
}

struct RightDrag {
    latest: (u64, u64),
    applied: (u64, u64),
    time: f64,
}

impl State {
    pub(crate) fn is_none(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }

    fn update_right(
        &mut self,
        ui: &egui::Ui,
        range: Option<(u64, u64)>,
        candidate: Option<(u64, u64)>,
        released: bool,
    ) -> Option<Action> {
        let now = ui.input(|i| i.time);
        if range.is_none() {
            self.right = None;
            return None;
        }
        if self.right.is_none() {
            let candidate = candidate.filter(|r| Some(*r) != range)?;
            if !released {
                self.right = Some(RightDrag {
                    latest: candidate,
                    applied: candidate,
                    time: now,
                });
            }
            return Some(Action::Start(candidate));
        }
        let drag = self.right.as_mut().unwrap();
        if let Some(candidate) = candidate {
            drag.latest = candidate;
        }
        let pending = drag.latest != drag.applied;
        let action = if pending && (released || now - drag.time >= RESTART_INTERVAL) {
            drag.applied = drag.latest;
            drag.time = now;
            Some(Action::Start(drag.latest))
        } else {
            if pending {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        (RESTART_INTERVAL - (now - drag.time)).max(0.0),
                    ));
            }
            (Some(drag.latest) != range).then_some(Action::Preview(drag.latest))
        };
        if released {
            self.right = None;
        }
        action
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Start((u64, u64)),
    Preview((u64, u64)),
    Clear,
}

pub(crate) fn apply(app: &mut crate::app::App, action: Action) {
    match action {
        Action::Start(range) => crate::play::transport::repeat::start(app, range),
        Action::Preview(range) => app.playback.repeat = Some(range),
        Action::Clear => crate::play::transport::repeat::clear(app),
    }
}

/// Range feedback follows the pointer; forced audio restarts are rate limited.
pub(crate) fn row(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    axis: &impl TickAxis,
    range: Option<(u64, u64)>,
    drag: &mut State,
) -> Option<Action> {
    let response = ui.interact(
        rect,
        ui.id().with("timeline-repeat"),
        egui::Sense::click_and_drag(),
    );
    let tick = |pos: egui::Pos2| axis.tick(pos.x.clamp(rect.left(), rect.right()) - rect.left());
    let completed = super::timeline_drag::completed(ui, rect);
    if response.drag_started_by(egui::PointerButton::Primary) || completed.is_some() {
        drag.left = completed
            .map(|p| p.0)
            .or_else(|| ui.input(|i| i.pointer.press_origin()))
            .map(tick);
        drag.right = None;
    }
    let preview = drag
        .left
        .zip(
            completed
                .map(|p| p.1)
                .or_else(|| response.interact_pointer_pos()),
        )
        .map(|(a, pos)| {
            let b = tick(pos);
            (a.min(b), a.max(b))
        });
    let candidate = if response.secondary_clicked()
        || response.dragged_by(egui::PointerButton::Secondary)
        || response.drag_stopped_by(egui::PointerButton::Secondary)
    {
        range
            .zip(response.interact_pointer_pos())
            .map(|((a, b), pos)| {
                let t = tick(pos);
                if t.abs_diff(a) <= t.abs_diff(b) {
                    (t.min(b.saturating_sub(1)), b)
                } else {
                    (a, t.max(a.saturating_add(1)))
                }
            })
    } else {
        None
    };
    let released = !ui.input(|i| i.pointer.secondary_down());
    let right_action = drag.update_right(ui, range, candidate, released);
    let right_preview = candidate.or_else(|| drag.right.as_ref().map(|d| d.latest));
    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(40));
    if let Some((a, b)) = preview.or(right_preview).or(range) {
        let selected = egui::Rect::from_min_max(
            egui::pos2(rect.left() + axis.x(a), rect.top()),
            egui::pos2(rect.left() + axis.x(b), rect.bottom()),
        );
        ui.painter()
            .rect_filled(selected, 0.0, RANGE.gamma_multiply(0.3));
        for (x, label, align) in [
            (selected.left(), "A", egui::Align2::LEFT_CENTER),
            (selected.right(), "B", egui::Align2::RIGHT_CENTER),
        ] {
            ui.painter()
                .vline(x, rect.y_range(), egui::Stroke::new(1.0_f32, RANGE));
            ui.painter().text(
                egui::pos2(x, rect.center().y),
                align,
                label,
                egui::FontId::monospace(12.0),
                RANGE,
            );
        }
    }
    let label_rect = rect.intersect(ui.clip_rect());
    ui.painter().text(
        label_rect.center(),
        egui::Align2::CENTER_CENTER,
        "A-B repeat",
        egui::FontId::monospace(12.0),
        RANGE,
    );
    if response.drag_stopped_by(egui::PointerButton::Primary) || completed.is_some() {
        drag.left = None;
        return preview.filter(|(a, b)| a < b).map(Action::Start);
    }
    right_action.or_else(|| response.clicked().then_some(Action::Clear))
}

#[cfg(test)]
mod tests;
