//! Tempo layers share the note overview's time axis, with an independent song-wide BPM axis.
use crate::view::tempo_curve::{Axis, CURVE, TEMPO};
use cat_smf_editor::edit::reference_tempo::{ranks, Reference};
use cat_smf_editor::edit::tempo_curve::Curve;
use eframe::egui::{self, Color32};

pub(crate) const REFERENCE: Color32 = Color32::from_rgb(0xf9, 0x26, 0x72);
pub(crate) const REFERENCE_MISMATCH: Color32 = Color32::from_rgb(0xff, 0xa0, 0x40);

pub(crate) struct Tempos<'a> {
    pub events: &'a [(u64, f64)],
    pub curve: Option<&'a Curve>,
    pub references: &'a [Reference],
}

pub(crate) fn draw(painter: &egui::Painter, rect: egui::Rect, end: f32, data: Tempos<'_>) {
    let axis = Axis::new(
        data.events
            .iter()
            .map(|p| p.1)
            .chain(data.references.iter().map(|p| p.bpm))
            .chain(
                data.curve
                    .iter()
                    .flat_map(|c| c.points.iter().chain(&c.controls))
                    .map(|p| p.bpm),
            ),
    );
    let area = rect.shrink(2.0);
    let pos = |tick: f64, bpm: f64| {
        egui::pos2(
            rect.left() + tick as f32 / end * rect.width(),
            area.top() + axis.y(bpm, area.height()),
        )
    };
    let mut steps = Vec::new();
    for (i, &(tick, bpm)) in data.events.iter().enumerate() {
        let next = data
            .events
            .get(i + 1)
            .map_or(f64::from(end), |p| p.0 as f64);
        steps.extend([pos(tick as f64, bpm), pos(next, bpm)]);
    }
    if !steps.is_empty() {
        painter.add(egui::Shape::line(steps, egui::Stroke::new(1.0_f32, TEMPO)));
    }
    if let Some(curve) = data.curve {
        let samples = rect.width().ceil().max(1.0) as usize;
        let points = (0..=samples)
            .map(|i| {
                let tick = f64::from(end) * i as f64 / samples as f64;
                pos(tick, curve.bpm_at(tick))
            })
            .collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, CURVE)));
    }
    let current = ranks(data.references);
    for (i, point) in data.references.iter().enumerate() {
        let next = data
            .references
            .get(i + 1)
            .map_or(f64::from(end), |p| p.tick as f64);
        let color = if current[i] == point.rank {
            REFERENCE
        } else {
            REFERENCE_MISMATCH
        };
        painter.extend(egui::Shape::dashed_line(
            &[pos(point.tick as f64, point.bpm), pos(next, point.bpm)],
            egui::Stroke::new(1.0_f32, color),
            4.0,
            3.0,
        ));
    }
}
