//! Curves a track holds, drawn over its pane on the overview's time axis.
use super::*;
use crate::view::tempo_curve::{Axis, Roll, CURVE};
use cat_smf_editor::edit::parameter_curve::{self, Curve};
use cat_smf_editor::score::macro_curve;

/// Each track's macro variation curves and control curve; unreadable ones are left out.
pub(super) fn read(loaded: &Loaded) -> Vec<Vec<Curve>> {
    let mut controls = parameter_curve::read(&loaded.music)
        .map(|c| c.curves)
        .unwrap_or_default();
    loaded
        .music
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let mut curves = macro_curve::read(track).unwrap_or_default();
            curves.extend(controls.remove(&index));
            curves
        })
        .collect()
}

/// Each curve scaled to its own value range.
pub(super) fn draw(painter: &egui::Painter, rect: egui::Rect, scale: f32, curves: &[Curve]) {
    for curve in curves {
        let points = curve_line(rect, painter.clip_rect(), scale, curve);
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, CURVE)));
    }
}

/// The tempo curve screen's flattening, so knots and peaks stay put while scrolling.
fn curve_line(rect: egui::Rect, clip: egui::Rect, scale: f32, curve: &Curve) -> Vec<egui::Pos2> {
    let visible = clip.intersect(rect);
    if visible.width() <= 0.0 {
        return Vec::new();
    }
    let roll = Roll {
        rect: egui::Rect::from_x_y_ranges(rect.x_range(), rect.shrink(2.0).y_range()),
        scale,
        axis: Axis {
            low: curve.minimum,
            high: curve.maximum,
        },
    };
    crate::view::tempo_curve::curve_path::line(&roll, &curve.points, &curve.controls, visible)
}

#[cfg(test)]
mod tests;
