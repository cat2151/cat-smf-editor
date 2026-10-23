//! Track-local variation curves: variation increases rightwards, time downwards.
use cat_smf_editor::edit::parameter_curve::{Curve, Point};
use eframe::egui::{self, Color32};

pub(super) const COLOR: Color32 = crate::view::tempo_curve::CONTROL;

pub(super) fn draw(painter: &egui::Painter, rect: egui::Rect, end: f64, curves: &[Curve]) {
    // Include outlying controls without changing their shape or clipping overshoots.
    let (low, high) = curves.iter().filter(|c| !c.points.is_empty()).fold(
        (0.0_f64, 1.0_f64),
        |(low, high), c| {
            c.points.iter().chain(&c.controls).fold(
                (low.min(c.minimum), high.max(c.maximum)),
                |(low, high), p| (low.min(p.value), high.max(p.value)),
            )
        },
    );
    let area = rect.shrink2(egui::vec2(2.0, 0.0));
    let pos = |point: Point| {
        egui::pos2(
            area.left() + ((point.value - low) / (high - low)) as f32 * area.width(),
            rect.top() + (point.tick as f64 / end) as f32 * rect.height(),
        )
    };
    let stroke = egui::Stroke::new(1.0_f32, COLOR);
    for curve in curves {
        let Some(&first) = curve.points.first() else {
            continue;
        };
        painter.line_segment([pos(Point { tick: 0, ..first }), pos(first)], stroke);
        for (pair, control) in curve.points.windows(2).zip(&curve.controls) {
            painter.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
                [pos(pair[0]), pos(*control), pos(pair[1])],
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        let last = *curve.points.last().unwrap();
        if (last.tick as f64) < end {
            painter.line_segment([pos(last), egui::pos2(pos(last).x, rect.bottom())], stroke);
        }
    }
}
