//! Fixed edge bars and independent tempo-axis / timeline zoom.
use super::*;
use crate::view::timeline_zoom::{self, BAR};

pub(crate) const VARIATION_MARGIN: f64 = 0.15;

pub(super) const BAR_COLOR: Color32 = Color32::from_gray(75);

mod edges;
mod minimap;
mod paging;
mod wheel;
mod zoom_drag;
use wheel::Mode;

pub(crate) struct Viewport {
    /// Native scalar limits; absent for the existing tempo viewport.
    pub bounds: Option<Axis>,
    /// Variation axes stay centered and cannot enlarge beyond the initial fit.
    pub limit_to_fit: bool,
    pub axis: Option<Axis>,
    zoom: f32,
    pub scroll: f32,
    pub width: f32,
    pub pending_scroll: Option<f32>,
    pub notes_drag: Option<f32>,
    pub tempo_rect: Option<egui::Rect>,
    edge_drag: Option<edges::Drag>,
    zoom_drag: Option<zoom_drag::Drag>,
    minimap_state: crate::view::timeline_minimap::State,
    pub minimap_rect: Option<egui::Rect>,
    pub seek: Option<u64>,
    horizontal_bar: Option<egui::Rect>,
    vertical_scrollbar: Option<egui::Rect>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            bounds: None,
            limit_to_fit: false,
            axis: None,
            zoom: 1.0,
            scroll: 0.0,
            width: 0.0,
            pending_scroll: None,
            notes_drag: None,
            tempo_rect: None,
            edge_drag: None,
            zoom_drag: None,
            minimap_state: Default::default(),
            minimap_rect: None,
            seek: None,
            horizontal_bar: None,
            vertical_scrollbar: None,
        }
    }
}

impl Viewport {
    pub fn scale_layout(&self, layout: &mut Layout) {
        layout.scale *= self.zoom;
        layout.start *= self.zoom;
        layout.size.x *= self.zoom;
        for (x, _) in &mut layout.beats {
            *x *= self.zoom;
        }
        for note in layout
            .notes
            .iter_mut()
            .map(|(_, note)| note)
            .chain(&mut layout.references)
        {
            note.min.x *= self.zoom;
            note.max.x *= self.zoom;
        }
    }

    /// Bars stay at the viewport edges, including when the timeline is scrolled.
    pub fn bars(&mut self, ui: &mut egui::Ui, base: Axis, enabled: bool) -> egui::Rect {
        let mut mode = Mode::TimeScroll;
        let (outer, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let inner = egui::Rect::from_min_max(outer.min + egui::vec2(BAR, BAR), outer.max);
        let bars = [
            egui::Rect::from_min_max(outer.min, egui::pos2(outer.right(), inner.top())),
            egui::Rect::from_min_max(
                egui::pos2(outer.left(), inner.top()),
                egui::pos2(inner.left(), inner.bottom()),
            ),
        ];
        for (edge, rect) in [0, 2].into_iter().zip(bars) {
            let response = ui.interact(
                rect,
                ui.id().with(("tempo-zoom", edge)),
                egui::Sense::click_and_drag(),
            );
            let color = if response.hovered() { POINT } else { BAR_COLOR };
            ui.painter().rect_filled(rect.shrink(2.0), 2.0, color);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "zoom",
                egui::FontId::monospace(if edge < 2 { 11.0 } else { 9.0 }),
                if response.hovered() {
                    Color32::BLACK
                } else {
                    Color32::from_gray(220)
                },
            );
            self.drag_zoom(ui, rect, base, enabled, edge, &response);
            if enabled
                && response.hovered()
                && ui.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO)
            {
                mode = if edge == 0 {
                    Mode::TimeZoom
                } else {
                    Mode::BpmZoom
                };
            }
            if enabled
                && (response.clicked() || response.secondary_clicked())
                && let Some(pos) = response.interact_pointer_pos()
            {
                let anchor = if edge < 2 {
                    (pos.x - inner.left() - HIT).clamp(0.0, self.width)
                } else {
                    let panel = self.tempo_rect.unwrap_or(inner);
                    ((pos.y - panel.top()) / panel.height()).clamp(0.0, 1.0)
                };
                self.zoom_edge(edge, response.clicked(), base, anchor);
            }
        }
        self.wheel(ui, outer, base, enabled && self.zoom_drag.is_none(), mode);
        inner
    }

    pub(super) fn zoom_edge(&mut self, edge: usize, enlarge: bool, base: Axis, anchor: f32) {
        let click = f64::from(timeline_zoom::CLICK);
        let factor = if enlarge { click } else { 1.0 / click };
        if edge < 2 {
            self.zoom_time(self.zoom * factor as f32, anchor);
        } else {
            self.zoom_bpm(base, factor, anchor);
        }
    }

    fn zoom_bpm(&mut self, base: Axis, factor: f64, anchor: f32) {
        if self.limit_to_fit {
            let axis = self.axis.unwrap_or(base);
            let range = self.bounds();
            let minimum_span = (base.high - base.low) * (1.0 + 2.0 * VARIATION_MARGIN);
            let span = ((axis.high - axis.low) / factor)
                .clamp(minimum_span, *range.end() - *range.start());
            let center = (base.low + base.high) / 2.0;
            self.axis = Some(Axis {
                low: center - span / 2.0,
                high: center + span / 2.0,
            });
            return;
        }
        let axis = self.axis.unwrap_or(base);
        let center = axis.high - f64::from(anchor) * (axis.high - axis.low);
        let range = self.bounds();
        let minimum_span = self.bounds.map_or(0.01, |b| (b.high - b.low) * 1e-6);
        let span =
            ((axis.high - axis.low) / factor).clamp(minimum_span, *range.end() - *range.start());
        let low =
            (center - span / 2.0).clamp(*range.start(), (*range.end() - span).max(*range.start()));
        self.axis = Some(Axis {
            low,
            high: (low + span).min(*range.end()),
        });
    }

    pub(crate) fn zoom_time(&mut self, zoom: f32, anchor: f32) {
        let next = timeline_zoom::clamp(zoom);
        self.pending_scroll = Some(timeline_zoom::rescroll(
            self.scroll,
            anchor,
            self.zoom,
            next,
            self.width,
        ));
        self.zoom = next;
    }

    pub(crate) fn time_zoom(&self) -> f32 {
        self.zoom
    }

    fn bounds(&self) -> std::ops::RangeInclusive<f64> {
        self.bounds
            .map_or_else(tempo_curve::bpm_range, |b| b.low..=b.high)
    }

    pub fn notes(&mut self, ui: &mut egui::Ui, rect: egui::Rect, scale: f32) -> Option<u64> {
        let response = ui.interact(
            rect,
            crate::view::command_input::preview_id(ui.ctx(), egui::Id::new("tempo-notes")),
            egui::Sense::click_and_drag(),
        );
        let local = |p: egui::Pos2| (p.x - rect.left()).max(0.0);
        self.adjust_edge(ui, rect, scale, &response);
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.notes_drag = ui.input(|i| i.pointer.press_origin()).map(local);
        }
        if let Some((a, pos)) = self.notes_drag.zip(response.interact_pointer_pos()) {
            let b = local(pos);
            let selected = egui::Rect::from_x_y_ranges(
                rect.left() + a.min(b)..=rect.left() + a.max(b),
                rect.y_range(),
            );
            ui.painter()
                .rect_filled(selected, 0.0, TEMPO.gamma_multiply(0.25));
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                self.notes_drag = None;
                if (a - b).abs() > 2.0 {
                    self.zoom_time(
                        self.zoom * self.width * timeline_zoom::FIT / (a - b).abs(),
                        (a + b) / 2.0 - self.scroll,
                    );
                }
            }
        }
        response.clicked().then(|| {
            let x = local(response.interact_pointer_pos().unwrap());
            (x / scale).round() as u64
        })
    }

    pub fn vertical_bar(&mut self, ui: &mut egui::Ui, rect: egui::Rect, base: Axis) {
        self.tempo_rect = Some(rect);
        if self.limit_to_fit {
            self.vertical_scrollbar = None;
            return;
        }
        let axis = self.axis.unwrap_or(base);
        let span = axis.high - axis.low;
        let bounds = self.bounds();
        let low = (base.low - span).min(axis.low).max(*bounds.start());
        let high = (base.high + span).max(axis.high).min(*bounds.end());
        let content_height = rect.height() * ((high - low) / span) as f32;
        let offset = ((high - axis.high) / span) as f32 * rect.height();
        let bar = egui::Rect::from_min_max(
            egui::pos2(rect.right() + HIT + 2.0, rect.top()),
            egui::pos2(rect.right() + HIT + 16.0, rect.bottom()),
        );
        self.vertical_scrollbar = Some(bar);
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
            ui.spacing_mut().scroll.floating = false;
            let output = egui::ScrollArea::vertical()
                .id_salt("tempo-bpm-scroll")
                .scroll_bar_rect(bar.shrink2(egui::vec2(0.0, timeline_zoom::BUTTON)))
                .auto_shrink([false, false])
                .drag_to_scroll(false)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .vertical_scroll_offset(offset)
                .show(ui, |ui| {
                    ui.set_min_height(content_height);
                });
            if (output.state.offset.y - offset).abs() > 0.01 {
                let top = high - f64::from(output.state.offset.y / rect.height()) * span;
                self.axis = Some(Axis {
                    low: top - span,
                    high: top,
                });
            }
        });
        if let Some(direction) = timeline_zoom::page_buttons(ui, bar, true, "tempo-page") {
            self.apply_wheel(
                -direction * rect.height() * timeline_zoom::PAGE,
                base,
                Mode::BpmScroll,
            );
        }
    }
}

pub(crate) fn minimap_tempos<'a>(
    events: &'a [(u64, f64)],
    curve: Option<&'a Curve>,
    references: &'a [cat_smf_editor::edit::reference_tempo::Reference],
) -> minimap::overview::Tempos<'a> {
    minimap::overview::Tempos {
        events,
        curve,
        references,
    }
}
