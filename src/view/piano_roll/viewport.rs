//! Fixed zoom zones and independent time / pitch navigation.
use super::*;

const BAR: f32 = 24.0;
pub(crate) const BUTTON: f32 = 16.0;
const ACTIVE: Color32 = Color32::from_rgb(0xe6, 0xdb, 0x74);

#[derive(Default)]
pub(crate) struct Viewport {
    pub seek: Option<u64>,
    zoom: egui::Vec2,
    pub pending: Option<egui::Vec2>,
    pub(crate) scroll: egui::Vec2,
    pub(super) panel: Option<egui::Rect>,
    pub(super) bars: [Option<egui::Rect>; 2],
    drag: Option<(usize, f32, egui::Rect)>,
    pub(crate) minimap_state: crate::view::timeline_minimap::State,
    pub(super) zoom_bars: [Option<egui::Rect>; 2],
    pub(crate) panes: usize,
}

impl Viewport {
    fn axis_length(&self, axis: usize) -> f32 {
        self.panel.map_or(0.0, |p| {
            p.size()[axis]
                / if axis == 1 {
                    self.panes.max(1) as f32
                } else {
                    1.0
                }
        })
    }

    pub fn minimap(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        layout: &Layout,
        tempos: crate::view::timeline_minimap::tempos::Tempos<'_>,
        playing: Option<u64>,
    ) -> (egui::Rect, bool) {
        use crate::view::timeline_minimap::{Action, View};
        let zoom = self.zoom().x;
        let (area, action) = self.minimap_state.show(
            ui,
            area,
            layout,
            View {
                end: layout.size.x,
                playing: playing.map(|tick| tick as f32 * layout.scale),
                zoom,
                scroll: self.scroll.x,
                width: self.axis_length(0),
            },
            |painter, rect| {
                crate::view::timeline_minimap::tempos::draw(
                    painter,
                    rect,
                    layout.size.x / layout.scale,
                    tempos,
                )
            },
        );
        match action {
            Some(Action::Center(center)) => {
                self.seek = Some((center / layout.scale).round().max(0.0) as u64);
                let mut offset = self.pending.unwrap_or(self.scroll);
                offset.x = (center * zoom - self.axis_length(0) / 2.0).max(0.0);
                self.pending = Some(offset);
            }
            Some(Action::Scroll(left)) => {
                let mut offset = self.pending.unwrap_or(self.scroll);
                offset.x = left * zoom;
                self.pending = Some(offset);
            }
            None => {}
        }
        (area, action.is_some())
    }

    pub(super) fn fit_time_range(&mut self, start: f32, end: f32, fraction: f32) {
        let span = (end - start).abs();
        if span > 0.0 {
            self.zoom_axis(
                0,
                self.axis_length(0) * fraction / span,
                (start + end) / 2.0 - self.scroll.x,
            );
        }
    }

    pub(crate) fn zoom(&self) -> egui::Vec2 {
        if self.zoom == egui::Vec2::ZERO {
            egui::Vec2::splat(1.0)
        } else {
            self.zoom
        }
    }

    pub fn scale_layout(&self, layout: &mut Layout) {
        let zoom = self.zoom();
        layout.size = layout.size * zoom;
        layout.start *= zoom.x;
        layout.scale *= zoom.x;
        for (x, _) in &mut layout.beats {
            *x *= zoom.x;
        }
        for y in &mut layout.staff {
            *y *= zoom.y;
        }
        for (_, note) in &mut layout.notes {
            note.min = (note.min.to_vec2() * zoom).to_pos2();
            note.max = (note.max.to_vec2() * zoom).to_pos2();
        }
    }

    fn zoom_axis(&mut self, axis: usize, factor: f32, anchor: f32) {
        if self.panel.is_none() {
            return;
        }
        let mut zoom = self.zoom();
        let bounds = if axis == 0 {
            (f32::MIN_POSITIVE, 4096.0)
        } else {
            (1.0 / 64.0, 64.0)
        };
        let next = (zoom[axis] * factor).clamp(bounds.0, bounds.1);
        let mut offset = self.pending.unwrap_or(self.scroll);
        offset[axis] = ((self.scroll[axis] + anchor) * next / zoom[axis]
            - self.axis_length(axis) / 2.0)
            .max(0.0);
        zoom[axis] = next;
        self.zoom = zoom;
        self.pending = Some(offset);
    }

    pub fn bars(&mut self, ui: &mut egui::Ui, enabled: bool) -> egui::Rect {
        let mut mode = (0, false);
        let (outer, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let inner = egui::Rect::from_min_max(outer.min + egui::vec2(BAR, BAR), outer.max);
        let zones = [
            egui::Rect::from_min_max(outer.min, egui::pos2(outer.right(), inner.top())),
            egui::Rect::from_min_max(
                egui::pos2(outer.left(), inner.top()),
                egui::pos2(inner.left(), inner.bottom()),
            ),
        ];
        for (axis, zone) in zones.into_iter().enumerate() {
            self.zoom_bars[axis] = Some(zone);
            let response = ui.interact(
                zone,
                ui.id().with(("roll-zoom", axis)),
                egui::Sense::click_and_drag(),
            );
            ui.painter().rect_filled(
                zone.shrink(2.0),
                2.0,
                if response.hovered() {
                    ACTIVE
                } else {
                    Color32::from_gray(75)
                },
            );
            ui.painter().text(
                zone.center(),
                egui::Align2::CENTER_CENTER,
                "zoom",
                egui::FontId::monospace(if axis == 0 { 11.0 } else { 9.0 }),
                if response.hovered() {
                    Color32::BLACK
                } else {
                    Color32::from_gray(220)
                },
            );
            if !enabled {
                self.drag = None;
                continue;
            }
            if response.hovered() && ui.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO) {
                mode = (axis, true);
            }
            let Some(panel) = self.panel else {
                continue;
            };
            let count = self.panes.max(1) as f32;
            let axis_panel = |pos: egui::Pos2| {
                if axis == 0 {
                    return panel;
                }
                let height = panel.height() / count;
                let slot = ((pos.y - panel.top()) / height)
                    .floor()
                    .clamp(0.0, count - 1.0);
                egui::Rect::from_min_size(
                    panel.min + egui::vec2(0.0, slot * height),
                    egui::vec2(panel.width(), height),
                )
            };
            let local = |pos: egui::Pos2| {
                let panel = axis_panel(pos);
                (pos[axis] - panel.min[axis]).clamp(0.0, panel.size()[axis])
            };
            if (response.clicked() || response.secondary_clicked())
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.zoom_axis(
                    axis,
                    if response.clicked() {
                        3.0 / 2.0
                    } else {
                        2.0 / 3.0
                    },
                    local(pos),
                );
            }
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(pos) = ui.input(|i| i.pointer.press_origin())
            {
                self.drag = Some((axis, local(pos), axis_panel(pos)));
            }
            if let Some((_, start, panel)) = self.drag.filter(|(a, _, _)| *a == axis)
                && let Some(pos) = response.interact_pointer_pos()
            {
                let end = (pos[axis] - panel.min[axis]).clamp(0.0, panel.size()[axis]);
                let mut selected = zone;
                selected.min[axis] = panel.min[axis] + start.min(end);
                selected.max[axis] = panel.min[axis] + start.max(end);
                ui.painter()
                    .rect_filled(selected, 0.0, ACTIVE.gamma_multiply(0.5));
                if response.drag_stopped_by(egui::PointerButton::Primary)
                    && (start - end).abs() > 2.0
                {
                    self.zoom_axis(
                        axis,
                        panel.size()[axis] * 0.9 / (start - end).abs(),
                        (start + end) / 2.0,
                    );
                }
            }
            if response.drag_stopped() {
                self.drag = None;
            }
        }
        if ui.rect_contains_pointer(outer) {
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
            if enabled && self.drag.is_none() && delta != 0.0 {
                for (axis, bar) in self.bars.iter().enumerate() {
                    if bar.is_some_and(|r| ui.rect_contains_pointer(r)) {
                        mode = (axis, false);
                    }
                }
                let (axis, zoom) = mode;
                if zoom {
                    let anchor = self.axis_length(axis) / 2.0;
                    self.zoom_axis(axis, (delta * 0.01).exp(), anchor);
                } else {
                    let mut offset = self.scroll;
                    offset[axis] = (offset[axis] - delta).max(0.0);
                    self.pending = Some(offset);
                }
            }
        }
        inner
    }

    pub fn finish(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        panel: egui::Rect,
        offset: egui::Vec2,
    ) {
        self.scroll = offset;
        self.panel = Some(panel);
        let bars = [
            egui::Rect::from_min_max(
                panel.left_bottom(),
                egui::pos2(panel.right(), area.bottom()),
            ),
            egui::Rect::from_min_max(panel.right_top(), egui::pos2(area.right(), panel.bottom())),
        ];
        for (axis, bar) in bars.into_iter().enumerate() {
            self.bars[axis] = Some(bar);
            for end in [false, true] {
                let mut rect = bar;
                if end {
                    rect.min[axis] = rect.max[axis] - BUTTON;
                } else {
                    rect.max[axis] = rect.min[axis] + BUTTON;
                }
                let response = ui.interact(
                    rect,
                    ui.id().with(("roll-page", axis, end)),
                    egui::Sense::click(),
                );
                let visuals = ui.style().interact(&response);
                ui.painter()
                    .rect_filled(rect.shrink(1.0), 2.0, visuals.bg_fill);
                let direction = if end { 1.0 } else { -1.0 };
                let along = if axis == 0 {
                    egui::vec2(direction, 0.0)
                } else {
                    egui::vec2(0.0, direction)
                };
                let across = egui::vec2(-along.y, along.x);
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![
                        rect.center() + along * 4.0,
                        rect.center() - along * 3.0 + across * 3.0,
                        rect.center() - along * 3.0 - across * 3.0,
                    ],
                    visuals.fg_stroke.color,
                    egui::Stroke::NONE,
                ));
                if response.clicked() {
                    let mut next = offset;
                    next[axis] = (next[axis] + direction * self.axis_length(axis) * 0.5).max(0.0);
                    self.pending = Some(next);
                }
            }
        }
    }
}
