//! Fixed zoom zones and independent time / pitch navigation.
use crate::view::timeline_zoom::{self, ACTIVE, BAR};
use eframe::egui::{self, Color32};

#[derive(Default)]
pub(crate) struct Viewport {
    pub seek: Option<u64>,
    screen: Option<crate::view::timeline_controls::Screen>,
    horizontal_zoom: crate::view::timeline_controls::Zoom,
    zoom: egui::Vec2,
    pub pending: Option<egui::Vec2>,
    pub(crate) scroll: egui::Vec2,
    pub(crate) panel: Option<egui::Rect>,
    pub(crate) bars: [Option<egui::Rect>; 2],
    drag: Option<(usize, f32, egui::Rect)>,
    pub(crate) minimap_state: crate::view::timeline_minimap::State,
    pub(crate) zoom_bars: [Option<egui::Rect>; 2],
    pub(crate) panes: usize,
}

impl Viewport {
    pub fn suspend(&mut self) {
        self.minimap_state = Default::default();
        self.horizontal_zoom = Default::default();
        self.seek = None;
        self.pending = None;
        self.drag = None;
    }

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

    pub fn begin(&mut self, screen: crate::view::timeline_controls::Screen) {
        assert!(crate::view::timeline_controls::Screen::ALL.contains(&screen));
        if self.screen != Some(screen) {
            self.minimap_state = Default::default();
            self.horizontal_zoom = Default::default();
            self.seek = None;
            self.pending = None;
        }
        self.screen = Some(screen);
    }

    pub fn binding<'a>(
        &self,
        axis: &'a dyn crate::view::timeline_repeat::TickAxis,
        end: u64,
    ) -> crate::view::timeline_controls::Binding<'a> {
        crate::view::timeline_controls::Binding {
            screen: self
                .screen
                .expect("register timeline before drawing controls"),
            axis,
            viewport: self.panel.unwrap_or(egui::Rect::NOTHING),
            scroll: self.scroll.x,
            end,
            zoom_range: self.horizontal_zoom.range(),
        }
    }

    pub(crate) fn fit_time_range(&mut self, start: f32, end: f32, fraction: f32) {
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

    pub(crate) fn zoom_horizontal(&mut self, enlarge: bool) {
        self.zoom_axis(
            0,
            if enlarge {
                timeline_zoom::CLICK
            } else {
                1.0 / timeline_zoom::CLICK
            },
            self.axis_length(0) / 2.0,
        );
    }

    fn zoom_axis(&mut self, axis: usize, factor: f32, anchor: f32) {
        if self.panel.is_none() {
            return;
        }
        let mut zoom = self.zoom();
        let next = if axis == 0 {
            timeline_zoom::clamp(zoom[axis] * factor)
        } else {
            (zoom[axis] * factor).clamp(1.0 / 64.0, 64.0)
        };
        let mut offset = self.pending.unwrap_or(self.scroll);
        offset[axis] = if axis == 0 {
            crate::view::timeline_controls::zoom(
                self.scroll.x,
                self.axis_length(0),
                zoom.x,
                next,
                anchor,
            )
            .1
        } else {
            timeline_zoom::rescroll(
                self.scroll[axis],
                anchor,
                zoom[axis],
                next,
                self.axis_length(axis),
            )
        };
        zoom[axis] = next;
        self.zoom = zoom;
        self.pending = Some(offset);
    }

    pub fn bars(&mut self, ui: &mut egui::Ui, enabled: bool) -> egui::Rect {
        self.axis_bars(ui, enabled, false)
    }

    pub(crate) fn horizontal_bar(&mut self, ui: &mut egui::Ui, enabled: bool) -> egui::Rect {
        self.axis_bars(ui, enabled, true)
    }

    fn axis_bars(&mut self, ui: &mut egui::Ui, enabled: bool, horizontal: bool) -> egui::Rect {
        let mut mode = (0, false);
        let (outer, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let inner = egui::Rect::from_min_max(
            outer.min + egui::vec2(if horizontal { 0.0 } else { BAR }, BAR),
            outer.max,
        );
        let zones = [
            egui::Rect::from_min_max(outer.min, egui::pos2(outer.right(), inner.top())),
            egui::Rect::from_min_max(
                egui::pos2(outer.left(), inner.top()),
                egui::pos2(inner.left(), inner.bottom()),
            ),
        ];
        for (axis, zone) in zones
            .into_iter()
            .enumerate()
            .take(if horizontal { 1 } else { 2 })
        {
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
            if axis == 0 {
                let screen = self.screen.expect("register timeline before zoom input");
                if let Some(panel) = self.panel
                    && let Some((factor, anchor)) = self.horizontal_zoom.show(
                        ui,
                        zone,
                        (screen, panel),
                        enabled,
                        ACTIVE,
                        &response,
                    )
                {
                    self.zoom_axis(0, factor, anchor);
                }
                continue;
            }
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
                        timeline_zoom::CLICK
                    } else {
                        1.0 / timeline_zoom::CLICK
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
                        panel.size()[axis] * timeline_zoom::FIT / (start - end).abs(),
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
                    self.zoom_axis(axis, timeline_zoom::wheel(delta), anchor);
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
        self.finish_axes(ui, area, panel, offset, false);
    }

    fn finish_axes(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        panel: egui::Rect,
        offset: egui::Vec2,
        horizontal: bool,
    ) {
        let (panel, x) = crate::view::timeline_controls::finish(panel, offset.x);
        self.scroll = egui::vec2(x, offset.y);
        self.panel = Some(panel);
        let bars = [
            egui::Rect::from_min_max(
                panel.left_bottom(),
                egui::pos2(panel.right(), area.bottom()),
            ),
            egui::Rect::from_min_max(panel.right_top(), egui::pos2(area.right(), panel.bottom())),
        ];
        for (axis, bar) in bars
            .into_iter()
            .enumerate()
            .take(if horizontal { 1 } else { 2 })
        {
            self.bars[axis] = Some(bar);
            if let Some(direction) = timeline_zoom::page_buttons(ui, bar, axis == 1, "roll-page") {
                let mut next = offset;
                next[axis] = (next[axis]
                    + direction * self.axis_length(axis) * timeline_zoom::PAGE)
                    .max(0.0);
                self.pending = Some(next);
            }
        }
    }
}
