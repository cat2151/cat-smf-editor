//! Full-target overview gestures shared by registered horizontal timelines.
use eframe::egui::{self, Color32};

pub(crate) const COLOR: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);
pub(crate) const BACKGROUND: Color32 = Color32::from_gray(28);
const INSET: f32 = 8.0;
const HEIGHT_SHARE: f32 = 0.18;
const MIN_HEIGHT: f32 = 24.0;
const MAX_HEIGHT: f32 = 56.0;
const AFTER: f32 = 4.0;
const NOTE_TINT: f32 = 0.65;

/// A note's color here: its piano roll color, dimmed.
pub(crate) fn note_color(
    pitch: u8,
    key: Option<cat_smf_editor::score::key_signature::KeySignature>,
) -> Color32 {
    super::pitch_keyboard::roll_color(pitch, key).gamma_multiply(NOTE_TINT)
}

pub(crate) fn height(area: f32) -> f32 {
    (area * HEIGHT_SHARE).clamp(MIN_HEIGHT, MAX_HEIGHT)
}

/// Height of the minimap and `content` below it, without stretching the content.
pub(crate) fn height_with_content(content: f32) -> f32 {
    let content = content + AFTER;
    content + height(content / (1.0 - HEIGHT_SHARE))
}

#[derive(Default)]
pub(crate) struct State {
    pub rect: Option<egui::Rect>,
    drag: Option<f32>,
}

/// A navigation request in content coordinates, with a tick only for a click.
pub(crate) struct Navigation {
    pub seek: Option<u64>,
    pub scroll: f32,
}

impl State {
    /// Background rendering belongs to the adapter. All gestures use the same
    /// actual content binding, including nonuniform spacing and fixed-label gaps.
    pub fn show_on(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        binding: &super::timeline_controls::Binding<'_>,
        playing: Option<u64>,
        background: impl FnOnce(&egui::Painter, egui::Rect),
    ) -> Option<Navigation> {
        self.rect = Some(rect);
        if !rect.is_positive() {
            self.drag = None;
            return None;
        }
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
        painter.rect_filled(rect, 2.0, BACKGROUND);
        background(&painter, rect);
        let end = binding.end.max(1) as f32;
        let x = |tick: f64| rect.left() + (tick as f32 / end).clamp(0.0, 1.0) * rect.width();
        let (left, right) = binding.visible();
        let visible = if binding.end == 0 {
            rect
        } else {
            egui::Rect::from_x_y_ranges(x(left)..=x(right), rect.y_range())
        };
        painter.rect_filled(visible, 0.0, COLOR.gamma_multiply(0.12));
        painter.rect_stroke(
            visible,
            0.0,
            egui::Stroke::new(1.0_f32, COLOR),
            egui::StrokeKind::Inside,
        );
        let forecast = binding
            .forecast()
            .filter(|_| binding.end > 0)
            .map(|(left, right)| egui::Rect::from_x_y_ranges(x(left)..=x(right), rect.y_range()));
        if let Some(forecast) = forecast {
            painter.rect_stroke(
                forecast,
                0.0,
                egui::Stroke::new(1.5_f32, super::timeline_zoom::ACTIVE),
                egui::StrokeKind::Inside,
            );
        }
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new((binding.screen, "minimap")), rect);
            d.insert_temp(egui::Id::new((binding.screen, "visible")), visible);
            d.insert_temp(egui::Id::new((binding.screen, "forecast")), forecast);
        });
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new((binding.screen, "snapshot")),
                super::timeline_controls::Snapshot {
                    scroll: binding.scroll,
                    width: binding.width(),
                    end_x: binding.axis.x(binding.end),
                },
            )
        });
        let response = ui.interact(
            rect,
            ui.id().with((binding.screen, "timeline-minimap")),
            egui::Sense::click_and_drag(),
        );
        if !ui.is_enabled() {
            self.drag = None;
            return None;
        }
        let tick = |pos: egui::Pos2| {
            (f64::from(((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * end))
                .min(binding.end as f64)
        };
        let completed = super::timeline_drag::completed(ui, rect);
        if !ui.input(|i| i.pointer.primary_down())
            && !response.drag_stopped_by(egui::PointerButton::Primary)
            && completed.is_none()
        {
            self.drag = None;
        }
        if response.drag_started_by(egui::PointerButton::Primary) || completed.is_some() {
            self.drag = completed
                .map(|p| p.0)
                .or_else(|| ui.input(|i| i.pointer.press_origin()))
                .map(|pos| {
                    if visible.contains(pos) {
                        binding.axis.x_at(tick(pos)) - binding.scroll
                    } else {
                        binding.width() / 2.0
                    }
                });
        }
        let mut navigation = self
            .drag
            .zip(
                completed
                    .map(|p| p.1)
                    .or_else(|| response.interact_pointer_pos()),
            )
            .map(|(grab, pos)| Navigation {
                seek: None,
                scroll: binding.clamp(binding.axis.x_at(tick(pos)) - grab),
            });
        if self.drag.is_none()
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let tick = tick(pos);
            navigation = Some(Navigation {
                seek: Some(tick.round() as u64),
                scroll: binding.center(tick),
            });
        }
        if let Some(tick) = playing {
            painter.vline(
                x(tick as f64),
                rect.y_range(),
                egui::Stroke::new(2.0_f32, super::timeline_repeat::PLAYHEAD),
            );
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.drag = None;
        }
        navigation
    }
}

/// The standard fixed overview placement; adapters can provide a grid-specific rect.
pub(crate) fn area(area: egui::Rect) -> (egui::Rect, egui::Rect) {
    let rect = egui::Rect::from_min_max(
        area.min + egui::vec2(INSET, 2.0),
        egui::pos2(
            (area.right() - 16.0 - INSET).max(area.left() + INSET + 1.0),
            area.top() + height(area.height()),
        ),
    );
    (
        rect,
        egui::Rect::from_min_max(egui::pos2(area.left(), rect.bottom() + AFTER), area.max),
    )
}

pub(crate) mod tempos;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resizing_a_drag_to_zero_width_releases_it_without_invalid_navigation() {
        let ctx = egui::Context::default();
        let mut state = State {
            drag: Some(20.0),
            ..Default::default()
        };
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let binding = super::super::timeline_controls::Binding {
                    screen: super::super::timeline_controls::Screen::PianoRoll,
                    axis: &1.0_f32,
                    viewport: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 100.0)),
                    scroll: 0.0,
                    end: 100,
                    zoom_range: None,
                };
                assert!(state
                    .show_on(
                        ui,
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(0.0, 20.0)),
                        &binding,
                        None,
                        |_, _| panic!("empty overview has no background")
                    )
                    .is_none());
            });
        });
        assert!(state.drag.is_none());
    }
}
