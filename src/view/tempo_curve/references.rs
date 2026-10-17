//! Horizontal reference segments; dragging changes only their BPM.
use super::*;
use cat_smf_editor::edit::reference_tempo::{ranks, Reference as Point};

pub(super) const MISMATCH: Color32 = crate::view::timeline_minimap::tempos::REFERENCE_MISMATCH;

pub(super) const COLOR: Color32 = crate::view::timeline_minimap::tempos::REFERENCE;

#[derive(Default)]
pub(super) struct State {
    drag: Option<ReferenceDrag>,
}

struct ReferenceDrag {
    index: usize,
    origin_y: f32,
    initial_bpm: f64,
    axis: Axis,
}

impl State {
    pub fn axis(&self) -> Option<Axis> {
        self.drag.as_ref().map(|d| d.axis)
    }

    pub fn active(&self) -> Option<usize> {
        self.drag.as_ref().map(|d| d.index)
    }

    pub fn update(
        &mut self,
        roll: &Roll,
        response: &egui::Response,
        curve: Option<&Curve>,
        points: &[Point],
    ) -> (Vec<Point>, Option<Point>, bool) {
        let primary = egui::PointerButton::Primary;
        if response.drag_started_by(primary) {
            let origin = response.ctx.input(|i| i.pointer.press_origin());
            if let Some(pos) = origin.filter(|pos| curve.and_then(|c| roll.hit(c, *pos)).is_none())
            {
                self.drag = hit(roll, points, pos).map(|index| ReferenceDrag {
                    index,
                    origin_y: pos.y,
                    initial_bpm: points[index].bpm,
                    axis: roll.axis,
                });
            }
        }
        let mut preview = points.to_vec();
        let handled = self.drag.is_some();
        let mut changed = None;
        if let Some(drag) = &self.drag {
            if let (Some(point), Some(pos)) =
                (preview.get_mut(drag.index), response.interact_pointer_pos())
            {
                let bpm = drag.initial_bpm
                    + f64::from(drag.origin_y - pos.y) * (drag.axis.high - drag.axis.low)
                        / f64::from(roll.rect.height());
                point.bpm = bpm.clamp(
                    *tempo_curve::bpm_range().start(),
                    *tempo_curve::bpm_range().end(),
                );
                if response.drag_stopped_by(primary) && *point != points[drag.index] {
                    changed = Some(*point);
                }
            }
            response
                .ctx
                .set_cursor_icon(egui::CursorIcon::ResizeVertical);
        } else if response
            .hover_pos()
            .and_then(|pos| hit(roll, points, pos))
            .is_some()
        {
            response
                .ctx
                .set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if response.drag_stopped_by(primary) || !response.ctx.input(|i| i.pointer.primary_down()) {
            self.drag = None;
        }
        (preview, changed, handled)
    }
}

fn hit(roll: &Roll, points: &[Point], pos: egui::Pos2) -> Option<usize> {
    points.iter().enumerate().find_map(|(i, p)| {
        let start = roll.pinned(roll.pos(p.tick as f64, p.bpm));
        let end = points
            .get(i + 1)
            .map_or(roll.rect.right(), |p| roll.pos(p.tick as f64, p.bpm).x);
        (pos.x >= start.x && pos.x < end && (pos.y - start.y).abs() <= HIT).then_some(i)
    })
}

pub(super) fn draw(painter: &egui::Painter, roll: &Roll, points: &[Point], active: Option<usize>) {
    let painter = painter.with_clip_rect(painter.clip_rect().intersect(roll.rect.expand(HIT)));
    let current_ranks = ranks(points);
    for (i, p) in points.iter().enumerate() {
        let color = if current_ranks[i] == p.rank {
            COLOR
        } else {
            MISMATCH
        };
        let start = roll.pinned(roll.pos(p.tick as f64, p.bpm));
        let end = points
            .get(i + 1)
            .map_or(roll.rect.right(), |p| roll.pos(p.tick as f64, p.bpm).x);
        let left = start.x.max(painter.clip_rect().left());
        let right = end.min(painter.clip_rect().right());
        if left >= right {
            continue;
        }
        let stroke = egui::Stroke::new(if active == Some(i) { 3.0 } else { 1.5_f32 }, color);
        painter.extend(egui::Shape::dashed_line(
            &[egui::pos2(left, start.y), egui::pos2(right, start.y)],
            stroke,
            7.0,
            4.0,
        ));
        roll.edge_marker(
            &painter,
            egui::pos2(left + 8.0, roll.pos(0.0, p.bpm).y),
            color,
        );
        let label_y = start.y.clamp(roll.rect.top() + 12.0, roll.rect.bottom());
        painter.text(
            egui::pos2(
                left + if p.bpm < roll.axis.low || p.bpm > roll.axis.high {
                    16.0
                } else {
                    3.0
                },
                label_y,
            ),
            egui::Align2::LEFT_BOTTOM,
            format!("{:.1} BPM", p.bpm),
            LABEL_FONT,
            color,
        );
    }
}
