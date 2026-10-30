//! Shared timeline primitives with one read-only note pane and a bounded scalar pane.
use super::*;
use crate::view::{
    piano_roll::{self, Layout},
    tempo_curve::{self, Target as Kind},
    timeline_repeat as repeat, timeline_zoom,
};
use egui::{Color32, Pos2, Rect, Sense, Stroke};
type Changes = (
    Option<Curve>,
    Option<repeat::Action>,
    Option<(u64, bool)>,
    bool,
);

pub(super) fn show(
    ui: &mut egui::Ui,
    s: &mut State,
    c: &Curve,
    layout: &Layout,
    playing: Option<u64>,
    follow: bool,
    range: Option<(u64, u64)>,
) -> Changes {
    let mut area = ui.available_rect_before_wrap();
    area.max.y = (area.bottom() - 40.0).max(area.top() + 100.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        canvas(ui, s, c, layout, playing, follow, range)
    })
    .inner
}

fn canvas(
    ui: &mut egui::Ui,
    s: &mut State,
    c: &Curve,
    layout: &Layout,
    playing: Option<u64>,
    follow: bool,
    range: Option<(u64, u64)>,
) -> Changes {
    let mut edited = None;
    let mut action = None;
    let unzoomed_end = layout.size.x.max(
        c.points
            .last()
            .map_or(0.0, |p| p.tick as f32 * layout.scale + 96.0),
    );
    let base = tempo_curve::Axis {
        low: c.minimum,
        high: c.maximum,
    };
    s.viewport
        .begin(crate::view::timeline_controls::Screen::Parameter);
    s.viewport.limit_to_fit = s.variation_layer.is_some();
    s.viewport.bounds = Some(if s.viewport.limit_to_fit {
        let padding = (base.high - base.low) * 1.5;
        if s.viewport.axis.is_none() {
            let margin = (base.high - base.low) * tempo_curve::viewport::VARIATION_MARGIN;
            s.viewport.axis = Some(tempo_curve::Axis {
                low: base.low - margin,
                high: base.high + margin,
            });
        }
        tempo_curve::Axis {
            low: base.low - padding,
            high: base.high + padding,
        }
    } else {
        base
    });
    let area = s
        .viewport
        .bars(ui, base, !s.drag && s.repeat_drag.is_none());
    let time_scale = layout.scale * s.viewport.time_zoom();
    let timeline = s
        .viewport
        .binding(&time_scale, (unzoomed_end / layout.scale).round() as u64);
    let (area, navigated) = s.viewport.minimap(
        ui,
        area,
        layout,
        &timeline,
        tempo_curve::viewport::minimap_tempos(&[], None, &[]),
        playing,
    );
    let mut seek = s.viewport.seek.take().map(|t| (t, false));
    let zoom = s.viewport.time_zoom();
    let scale = layout.scale * zoom;
    let axis = s.viewport.axis.unwrap_or(base);
    let mut panel = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        ui.spacing_mut().scroll.floating = false;
        ui.set_max_width((area.width() - 16.0).max(1.0));
        let scrollbar =
            Rect::from_min_size(area.min, egui::vec2(ui.available_width(), area.height()));
        let mut scroll = egui::ScrollArea::horizontal()
            .id_salt("parameter-curve-scroll")
            .scroll_bar_rect(scrollbar.shrink2(egui::vec2(timeline_zoom::BUTTON, 0.0)))
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
        let following =
            crate::view::timeline_follow::Follow::new(ui.ctx(), follow && !navigated, playing)
                .tick();
        if let Some(t) = following {
            s.viewport.pending_scroll = None;
            scroll = scroll.horizontal_scroll_offset(
                (t as f32 * scale - (ui.available_width() - 16.0) / 2.0).max(0.0),
            );
        } else if let Some(offset) = s.viewport.pending_scroll.take() {
            scroll = scroll.horizontal_scroll_offset(offset);
        }
        let output = scroll.show_viewport(ui, |ui, viewport| {
            let (whole, _) = ui.allocate_exact_size(
                egui::vec2(
                    (unzoomed_end * zoom).max(viewport.width()),
                    viewport.height().max(100.0),
                ),
                Sense::hover(),
            );
            let row = Rect::from_min_size(whole.min, egui::vec2(whole.width(), repeat::HEIGHT));
            action = timeline.repeat(
                ui,
                row.shrink2(egui::vec2(8.0, 0.0)),
                range,
                &mut s.repeat_drag,
            );
            let panels = Rect::from_min_max(whole.min + egui::vec2(0.0, repeat::HEIGHT), whole.max)
                .shrink2(egui::vec2(8.0, 8.0));
            let notes = Rect::from_min_max(
                panels.min,
                egui::pos2(panels.right(), panels.center().y - 4.0),
            );
            let rect = Rect::from_min_max(
                egui::pos2(panels.left(), panels.center().y + 4.0),
                panels.max,
            );
            panel = Some(rect);
            let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(whole));
            let beats: Vec<_> = layout.beats.iter().map(|&(x, b)| (x * zoom, b)).collect();
            piano_roll::draw_beats(&painter, notes, &beats);
            piano_roll::draw_beats(&painter, rect, &beats);
            for ((_, note), color) in layout.notes.iter().zip(&layout.colors) {
                let pos = |p: Pos2| {
                    notes.min + egui::vec2(p.x * zoom, p.y / layout.size.y * notes.height())
                };
                painter.rect_filled(
                    Rect::from_min_max(pos(note.min), pos(note.max)),
                    1.0,
                    *color,
                );
            }
            if let Some(tick) = s.viewport.notes(ui, notes, scale) {
                seek = Some((tick, true));
            }
            let painter = painter.with_clip_rect(painter.clip_rect().intersect(rect));
            let response = ui.interact(
                rect,
                ui.id().with("parameter-points"),
                Sense::click_and_drag(),
            );
            let roll = tempo_curve::Roll { rect, scale, axis };
            let pos = |p: Point| roll.pos(p.tick as f64, p.value);
            let at = |p: Pos2| Point {
                tick: ((p.x - rect.left()) / scale).round().max(0.0) as u64,
                value: axis.low
                    + f64::from((rect.bottom() - p.y) / rect.height()) * (axis.high - axis.low),
            };
            if s.input.is_none() {
                let base = s.draft.as_ref().unwrap_or(c);
                let hit = response.hover_pos().and_then(|pointer| {
                    base.points
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (Kind::Point, i, p))
                        .chain(
                            base.controls
                                .iter()
                                .enumerate()
                                .map(|(i, p)| (Kind::Control, i, p)),
                        )
                        .find(|(_, _, p)| roll.pinned(pos(**p)).distance(pointer) <= 8.0)
                        .map(|(k, i, _)| (k, i))
                });
                let completed = crate::view::timeline_drag::completed(ui, rect);
                if response.drag_started() || completed.is_some() {
                    let origin = completed
                        .map(|p| p.0)
                        .or_else(|| ui.input(|i| i.pointer.press_origin()))
                        .unwrap_or(rect.min);
                    let hit = base
                        .points
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (Kind::Point, i, p))
                        .chain(
                            base.controls
                                .iter()
                                .enumerate()
                                .map(|(i, p)| (Kind::Control, i, p)),
                        )
                        .find(|(_, _, p)| roll.pinned(pos(**p)).distance(origin) <= 8.0)
                        .map(|(k, i, _)| (k, i));
                    let mut draft = c.clone();
                    s.selected = hit.or_else(|| {
                        let p = at(origin);
                        draft
                            .add(
                                p.tick,
                                if s.variation_layer.is_some() {
                                    p.value.clamp(c.minimum, c.maximum).round()
                                } else {
                                    p.value.clamp(c.minimum, c.maximum)
                                },
                            )
                            .ok()
                            .map(|i| (Kind::Point, i))
                    });
                    s.draft = Some(draft);
                    s.drag = true;
                }
                if s.drag
                    && let (Some((kind, index)), Some(pointer), Some(draft)) = (
                        s.selected,
                        completed
                            .map(|p| p.1)
                            .or_else(|| response.interact_pointer_pos()),
                        s.draft.as_mut(),
                    )
                {
                    let p = at(pointer);
                    if kind == Kind::Point {
                        let low = if index > 0 {
                            draft.points[index - 1].tick.saturating_add(1)
                        } else {
                            0
                        };
                        let high = draft
                            .points
                            .get(index + 1)
                            .map_or(u64::MAX, |p| p.tick.saturating_sub(1));
                        let value = if s.variation_layer.is_some() {
                            draft.points[index].value
                        } else {
                            p.value.clamp(c.minimum, c.maximum)
                        };
                        let _ = draft.move_point(index, p.tick.clamp(low, high), value);
                    } else {
                        let value = if s.variation_layer.is_some() {
                            p.value
                        } else {
                            p.value.clamp(c.minimum, c.maximum)
                        };
                        let _ = draft.move_control(index, p.tick, value);
                    }
                }
                if response.drag_stopped() || completed.is_some() {
                    s.drag = false;
                    edited = s.draft.take().filter(|d| d != c);
                }
                if response.clicked() {
                    if let Some(hit) = hit {
                        s.selected = Some(hit);
                    } else if let Some(pointer) = response.interact_pointer_pos() {
                        let mut next = c.clone();
                        let p = at(pointer);
                        if let Ok(index) = next.add(
                            p.tick,
                            if s.variation_layer.is_some() {
                                p.value.clamp(c.minimum, c.maximum).round()
                            } else {
                                p.value.clamp(c.minimum, c.maximum)
                            },
                        ) {
                            s.selected = Some((Kind::Point, index));
                            edited = Some(next);
                        }
                    }
                }
                if response.secondary_clicked()
                    && let Some((Kind::Point, index)) = hit
                {
                    let mut next = c.clone();
                    if next.remove(index).is_ok() {
                        s.selected = None;
                        edited = Some(next);
                    }
                }
            }
            let preview = s.draft.as_ref().unwrap_or(c);
            if let Some(first) = preview.points.first() {
                painter.line_segment(
                    [egui::pos2(rect.left(), pos(*first).y), pos(*first)],
                    Stroke::new(2.0_f32, tempo_curve::CURVE),
                );
                for (pair, control) in preview.points.windows(2).zip(&preview.controls) {
                    painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
                        [pos(pair[0]), pos(*control), pos(pair[1])],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0_f32, tempo_curve::CURVE),
                    ));
                    painter.line_segment(
                        [pos(pair[0]), pos(*control)],
                        Stroke::new(0.5_f32, tempo_curve::CONTROL.gamma_multiply(0.4)),
                    );
                    painter.line_segment(
                        [pos(*control), pos(pair[1])],
                        Stroke::new(0.5_f32, tempo_curve::CONTROL.gamma_multiply(0.4)),
                    );
                }
                let last = pos(*preview.points.last().unwrap());
                painter.line_segment(
                    [last, egui::pos2(rect.right(), last.y)],
                    Stroke::new(2.0_f32, tempo_curve::CURVE),
                );
            }
            for (kind, points, color) in [
                (Kind::Point, &preview.points, tempo_curve::POINT),
                (Kind::Control, &preview.controls, tempo_curve::CONTROL),
            ] {
                for (index, p) in points.iter().enumerate() {
                    let center = roll.pinned(pos(*p));
                    roll.edge_marker(&painter, pos(*p), color);
                    if s.selected == Some((kind, index)) {
                        painter.circle_stroke(center, 9.0, Stroke::new(1.0_f32, Color32::WHITE));
                    }
                    if kind == Kind::Point {
                        painter.circle_filled(center, 5.0, color);
                    } else {
                        painter.rect_filled(
                            Rect::from_center_size(center, egui::vec2(8.0, 8.0)),
                            0.0,
                            color,
                        );
                    }
                }
            }
            for (v, label) in &s.labels {
                let y = rect.bottom()
                    - ((v - axis.low) / (axis.high - axis.low)) as f32 * rect.height();
                painter.hline(
                    rect.x_range(),
                    y,
                    Stroke::new(0.5_f32, Color32::from_gray(60)),
                );
                painter.text(
                    egui::pos2(ui.clip_rect().left() + 4.0, y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::monospace(11.0),
                    Color32::GRAY,
                );
            }
            if s.labels.is_empty() {
                for v in [c.minimum, c.maximum] {
                    painter.text(
                        egui::pos2(
                            ui.clip_rect().left() + 4.0,
                            pos(Point { tick: 0, value: v }).y,
                        ),
                        egui::Align2::LEFT_CENTER,
                        v.to_string(),
                        egui::FontId::monospace(11.0),
                        Color32::GRAY,
                    );
                }
            }
            if let Some(tick) = playing {
                ui.painter()
                    .with_clip_rect(ui.clip_rect().intersect(whole))
                    .vline(
                        rect.left() + tick as f32 * scale,
                        whole.y_range(),
                        Stroke::new(1.5_f32, repeat::PLAYHEAD),
                    );
            }
        });
        if let Some(mut rect) = panel {
            rect.min.x = output.inner_rect.left() + 8.0;
            rect.max.x = output.inner_rect.right() - 8.0;
            s.viewport.vertical_bar(ui, rect, base);
        }
        s.viewport.horizontal.finish(
            output.inner_rect.shrink2(egui::vec2(8.0, 0.0)),
            output.state.offset.x,
        );
        s.viewport.horizontal_pages(
            ui,
            Rect::from_min_max(
                output.inner_rect.left_bottom(),
                egui::pos2(output.inner_rect.right(), area.bottom()),
            ),
        );
    });
    (edited, action, seek, navigated)
}
