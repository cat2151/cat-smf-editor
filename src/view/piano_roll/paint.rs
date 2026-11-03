//! Paints one roll: beats, staff, references, notes and the hovered part.
use super::*;

/// The chord that Chord timing edits.
const CHORD_COLOR: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);

/// `moved` replaces one rectangle of `layout.notes` while it is dragged.
/// `hover` is the index into `layout.notes` and the part to paint in its hover color.
pub(super) fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &Layout,
    moved: Option<(usize, egui::Rect)>,
    hover: Option<(usize, hover::Part)>,
    chord: &[usize],
) {
    let origin = rect.min.to_vec2();
    draw_beats(painter, rect, &layout.beats);
    for &y in &layout.staff {
        draw_horizontal_grid_line(painter, rect, rect.top() + y, STAFF_COLOR);
    }
    let note = |i: usize| match moved {
        Some((m, rect)) if m == i => rect,
        _ => layout.notes[i].1,
    };
    for &note in &layout.references {
        painter.rect_filled(note.translate(origin), 0.0, REFERENCE_COLOR);
    }
    for i in 0..layout.notes.len() {
        let (fill, stroke) = if chord.contains(&layout.notes[i].0) {
            (CHORD_COLOR, egui::Stroke::new(2.0_f32, Color32::WHITE))
        } else {
            (
                layout.colors[i],
                egui::Stroke::new(0.5_f32, layout.colors[i].gamma_multiply(0.5)),
            )
        };
        painter.rect(
            note(i).translate(origin),
            0.0,
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
    }
    if let Some((i, part)) = hover {
        painter.rect(
            hover::part_rect(note(i), part).translate(origin),
            0.0,
            part.color(),
            egui::Stroke::new(1.0_f32, Color32::BLACK),
            egui::StrokeKind::Inside,
        );
    }
    // Paint after the hover highlight so the mora stays readable while editing.
    lyrics::draw(
        painter,
        layout.labels(|r| r.translate(origin), |i| note(i).translate(origin)),
    );
}
