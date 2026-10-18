//! Beams joining eighth and shorter notes that follow each other within one beat.
use super::{
    glyphs,
    score::{self, Note, Staff},
};
use eframe::egui::{self, Color32};
use std::ops::Range;

/// Minimum stem from the head nearest the beam, in gaps.
const STEM: f32 = 3.25;
const THICKNESS: f32 = 0.5;
/// Distance between the outer edges of two beam levels, in gaps.
const LEVEL: f32 = 0.75;

/// Index ranges of `notes` (ascending by tick) to beam; notes sharing a tick form one column.
/// `beat` names the beat holding a tick.
pub(super) fn groups(
    notes: &[Note],
    ppqn: u16,
    beat: impl Fn(u64) -> (usize, u64),
) -> Vec<Range<usize>> {
    let mut columns = Vec::new();
    let mut start = 0;
    for column in notes.chunk_by(|a, b| a.tick == b.tick) {
        columns.push(start..start + column.len());
        start += column.len();
    }
    let short = |c: &Range<usize>| notes[c.clone()].iter().all(|n| n.value <= -1);
    // No rest between: the next column starts within the time this column's value shows.
    let joins = |a: &Range<usize>, b: &Range<usize>| {
        let (from, to) = (notes[a.start].tick, notes[b.start].tick);
        let value = notes[a.clone()].iter().map(|n| n.value).max().unwrap_or(0);
        let shown = f64::from(ppqn) * 2f64.powi(value);
        short(b) && beat(from) == beat(to) && (to - from) as f64 <= shown * 1.5
    };
    let mut groups = Vec::new();
    let mut run: Vec<Range<usize>> = Vec::new();
    for column in columns {
        if run.last().is_some_and(|last| joins(last, &column)) {
            run.push(column);
            continue;
        }
        if let [first, .., last] = run.as_slice() {
            groups.push(first.start..last.end);
        }
        run.clear();
        if short(&column) {
            run.push(column);
        }
    }
    if let [first, .., last] = run.as_slice() {
        groups.push(first.start..last.end);
    }
    groups
}

/// Stems and beams of one group whose heads are drawn without stems; each column takes the
/// color of its first note.
pub(super) fn draw(
    staff: &Staff,
    painter: &egui::Painter,
    notes: &[Note],
    x: impl Fn(u64) -> f32,
    color: impl Fn(&Note) -> Color32,
) {
    let gap = staff.gap;
    // The note farthest from its staff's middle line points every stem.
    let up = notes
        .iter()
        .map(|n| n.spelled.step - score::middle(n.spelled.step))
        .max_by_key(|d| d.abs())
        .is_some_and(|d| d < 0);
    // Screen direction from the heads toward the beam.
    let toward = if up { -1.0 } else { 1.0 };
    let columns: Vec<Column> = notes
        .chunk_by(|a, b| a.tick == b.tick)
        .map(|column| {
            let steps = column.iter().map(|n| n.spelled.step);
            let (high, low) = (steps.clone().max().unwrap_or(0), steps.min().unwrap_or(0));
            let (near, far) = if up { (high, low) } else { (low, high) };
            Column {
                x: x(column[0].tick) + glyphs::stem_x(up) * gap,
                far: staff.y(far),
                needed: staff.y(near) + toward * STEM * gap,
                beams: column
                    .iter()
                    .map(|n| -n.value)
                    .max()
                    .unwrap_or(1)
                    .clamp(1, 3),
                color: color(&column[0]),
            }
        })
        .collect();
    let (Some(first), Some(last)) = (columns.first(), columns.last()) else {
        return;
    };
    let rise = (last.needed - first.needed).clamp(-gap, gap);
    let slope = rise / (last.x - first.x).max(1e-3);
    let line = |x: f32| first.needed + slope * (x - first.x);
    // Slide the beam until every stem reaches its minimum length and the shortest just does.
    let slack = columns
        .iter()
        .map(|c| toward * (line(c.x) - c.needed))
        .fold(f32::INFINITY, f32::min);
    let beam = |x: f32| line(x) - toward * slack;
    let stem = egui::Stroke::new((glyphs::STEM * gap).max(1.0), Color32::TRANSPARENT);
    for c in &columns {
        painter.line_segment(
            [
                egui::pos2(c.x, c.far + toward * 0.2 * gap),
                egui::pos2(c.x, beam(c.x)),
            ],
            egui::Stroke {
                color: c.color,
                ..stem
            },
        );
    }
    let bar = |from: f32, to: f32, level: i32, color: Color32| {
        let outer = |x: f32| beam(x) - toward * level as f32 * LEVEL * gap;
        let inner = |x: f32| outer(x) - toward * THICKNESS * gap;
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(from, outer(from)),
                egui::pos2(to, outer(to)),
                egui::pos2(to, inner(to)),
                egui::pos2(from, inner(from)),
            ],
            color,
            egui::Stroke::NONE,
        ));
    };
    for level in 0..3 {
        let has = |i: usize| columns.get(i).is_some_and(|c| c.beams > level);
        for (i, c) in columns.iter().enumerate() {
            if !has(i) {
                continue;
            }
            if has(i + 1) {
                bar(c.x, columns[i + 1].x, level, c.color);
            } else if !(i > 0 && has(i - 1)) {
                // A lone shorter note gets a stub toward its neighbor in the group.
                let neighbor = if i == 0 { &columns[1] } else { &columns[i - 1] };
                let toward_neighbor = neighbor.x - c.x;
                let length = toward_neighbor.signum() * gap.min(toward_neighbor.abs() / 2.0);
                bar(c.x.min(c.x + length), c.x.max(c.x + length), level, c.color);
            }
        }
    }
}

struct Column {
    /// Stem x.
    x: f32,
    /// y of the head farthest from the beam, where the stem starts.
    far: f32,
    /// y the beam must reach for the nearest head's stem to be long enough.
    needed: f32,
    beams: i32,
    color: Color32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::staff::notation;

    fn note(tick: u64, value: i32) -> Note {
        Note {
            tick,
            end: tick + 1,
            spelled: notation::spell(72, 0),
            value,
            accidental: None,
            color: Color32::WHITE,
        }
    }

    /// Beats are quarters from tick 0.
    fn groups_of(notes: &[Note]) -> Vec<Range<usize>> {
        groups(notes, 480, |tick| (0, tick / 480))
    }

    #[test]
    fn short_notes_join_within_a_beat_only() {
        let eighths: Vec<_> = (0..4).map(|i| note(i * 240, -1)).collect();
        assert_eq!(groups_of(&eighths), vec![0..2, 2..4]);
    }

    #[test]
    fn a_chord_is_one_column_and_a_rest_or_quarter_breaks_the_beam() {
        let notes = [note(0, -1), note(0, -1), note(240, -1)];
        assert_eq!(groups_of(&notes), vec![0..3]);
        // A sixteenth followed by a sixteenth rest, then a sixteenth: the rest shows.
        let gap = [note(0, -2), note(240, -2)];
        assert!(groups_of(&gap).is_empty());
        let quarter = [note(0, 0), note(480, -1), note(720, -1)];
        assert_eq!(groups_of(&quarter), vec![1..3]);
    }
}
