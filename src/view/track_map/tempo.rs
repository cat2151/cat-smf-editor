//! Quiet tempo backdrop: BPM increases rightwards, time downwards like the event list.
use crate::view::tempo_curve::CURVE;
use cat_smf_editor::edit::tempo_curve::{Curve, Point};
use eframe::egui::{self, Color32};

pub(super) fn draw(painter: &egui::Painter, rect: egui::Rect, end: f64, curve: &Curve) {
    let axis = crate::view::tempo_curve::Axis::new(
        curve.points.iter().chain(&curve.controls).map(|p| p.bpm),
    );
    let area = rect.shrink2(egui::vec2(2.0, 0.0));
    let pos = |point: Point| {
        egui::pos2(
            area.left() + ((point.bpm - axis.low) / (axis.high - axis.low)) as f32 * area.width(),
            rect.top() + (point.tick as f64 / end) as f32 * rect.height(),
        )
    };
    let stroke = egui::Stroke::new(1.0_f32, CURVE);
    let first = curve.points[0];
    let last = curve.points[curve.points.len() - 1];
    painter.line_segment([pos(Point { tick: 0, ..first }), pos(first)], stroke);
    for (points, control) in curve.points.windows(2).zip(&curve.controls) {
        painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
            [pos(points[0]), pos(*control), pos(points[1])],
            false,
            Color32::TRANSPARENT,
            stroke,
        ));
    }
    if (last.tick as f64) < end {
        let bottom = egui::pos2(pos(last).x, rect.bottom());
        painter.line_segment([pos(last), bottom], stroke);
    }
}
