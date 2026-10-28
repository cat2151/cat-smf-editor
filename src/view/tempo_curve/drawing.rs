use super::*;
use crate::view::piano_roll::{draw_horizontal_grid_line, STAFF_COLOR};

/// Both kinds of marks are directly draggable.
pub(super) fn draw(
    painter: &egui::Painter,
    roll: &Roll,
    notes_rect: egui::Rect,
    layout: &Layout,
    curve: Option<&Curve>,
    tempos: &[(u64, f64)],
    hover: Option<(Target, usize)>,
) {
    // Pitch and BPM have separate vertical coordinate systems and clipping regions.
    let notes_painter = painter.with_clip_rect(painter.clip_rect().intersect(notes_rect));
    draw_beats(&notes_painter, notes_rect, &layout.beats);
    let factor = notes_rect.height() / layout.size.y;
    for &y in &layout.staff {
        draw_horizontal_grid_line(
            &notes_painter,
            notes_rect,
            notes_rect.top() + y * factor,
            STAFF_COLOR,
        );
    }
    for (&note, color) in layout
        .references
        .iter()
        .map(|note| (note, super::super::piano_roll::REFERENCE_COLOR))
        .chain(
            layout
                .notes
                .iter()
                .zip(&layout.colors)
                .map(|((_, note), &color)| (note, color)),
        )
    {
        let mut note = note;
        note.min.y *= factor;
        note.max.y *= factor;
        notes_painter.rect_filled(note.translate(notes_rect.min.to_vec2()), 0.0, color);
    }
    let place = |mut note: egui::Rect| {
        note.min.y *= factor;
        note.max.y *= factor;
        note.translate(notes_rect.min.to_vec2())
    };
    crate::view::piano_roll::lyrics::draw(
        &notes_painter,
        layout.labels(place, |i| place(layout.notes[i].1)),
    );
    let painter = &painter.with_clip_rect(painter.clip_rect().intersect(roll.rect.expand(HIT)));
    let rect = roll.rect;
    draw_beats(painter, rect, &layout.beats);
    let label_x = painter.clip_rect().left().max(rect.left()) + 2.0;
    let step =
        GRID_STEP.max(((roll.axis.high - roll.axis.low) / 40.0 / GRID_STEP).ceil() * GRID_STEP);
    let mut bpm = (roll.axis.low / step).ceil() * step;
    while bpm <= roll.axis.high {
        let y = roll.pos(0.0, bpm).y;
        draw_horizontal_grid_line(painter, rect, y, GRID);
        painter.text(
            egui::pos2(label_x, y),
            egui::Align2::LEFT_BOTTOM,
            format!("{bpm:.0}"),
            LABEL_FONT,
            GRID_LABEL,
        );
        bpm += step;
    }
    draw_tempos(painter, roll, tempos);
    let Some(curve) = curve else {
        return;
    };
    let visible = rect.intersect(painter.clip_rect());
    let start = (visible.left() - rect.left()).max(0.0);
    let end = (visible.right() - rect.left()).max(start);
    let samples = ((end - start) / EDGE_MARKER_STEP).ceil() as usize;
    let line = super::curve_path::line(roll, &curve.points, &curve.controls, visible);
    painter.add(egui::Shape::line(line, egui::Stroke::new(2.0_f32, CURVE)));
    for x in 0..=samples {
        let tick = f64::from((start + x as f32 * EDGE_MARKER_STEP).min(end) / roll.scale);
        roll.edge_marker(painter, roll.pos(tick, curve.bpm_at(tick)), CURVE);
    }
    let size = |kind: Target, i: usize| {
        if hover == Some((kind, i)) {
            HOVER_SCALE
        } else {
            1.0
        }
    };
    for (i, &mark) in roll.marks(curve, Target::Control).iter().enumerate() {
        let square = egui::Rect::from_center_size(
            roll.pinned(mark),
            egui::Vec2::splat(CONTROL_SIDE * size(Target::Control, i)),
        );
        painter.rect_filled(square, 0.0, CONTROL);
        roll.edge_marker(painter, mark, CONTROL);
    }
    for (i, &mark) in roll.marks(curve, Target::Point).iter().enumerate() {
        let radius = POINT_RADIUS * size(Target::Point, i);
        painter.circle_filled(roll.pinned(mark), radius, POINT);
        roll.edge_marker(painter, mark, POINT);
    }
}

/// A step line holding each tempo until the next, and a dot at each tempo event.
fn draw_tempos(painter: &egui::Painter, roll: &Roll, tempos: &[(u64, f64)]) {
    let right = (roll.rect.right() - roll.rect.left()) / roll.scale;
    let mut steps = Vec::new();
    for (i, &(tick, bpm)) in tempos.iter().enumerate() {
        let next = tempos.get(i + 1).map_or(f64::from(right), |t| t.0 as f64);
        steps.push(roll.pinned(roll.pos(tick as f64, bpm)));
        steps.push(roll.pinned(roll.pos(next.max(tick as f64), bpm)));
        let left = roll.pos(tick as f64, bpm).x.max(painter.clip_rect().left());
        let end = roll.pos(next, bpm).x.min(painter.clip_rect().right());
        let mut x = left + 12.0;
        while x < end {
            roll.edge_marker(painter, egui::pos2(x, roll.pos(0.0, bpm).y), TEMPO);
            x += 32.0;
        }
    }
    if !steps.is_empty() {
        painter.add(egui::Shape::line(steps, egui::Stroke::new(1.0_f32, TEMPO)));
    }
    for &(tick, bpm) in tempos {
        painter.circle_filled(roll.pinned(roll.pos(tick as f64, bpm)), 2.0, TEMPO);
    }
}
