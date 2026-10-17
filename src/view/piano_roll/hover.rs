//! Which part of a note the pointer is on, and how that part is drawn.
use eframe::egui::{self, Color32};

/// One blink: the hover color for the first half, the note's own color for the second.
pub(crate) const BLINK_PERIOD: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Start,
    Middle,
    End,
}

impl Part {
    pub(crate) fn color(self) -> Color32 {
        match self {
            Self::Start => Color32::from_rgb(0xae, 0x81, 0xff),
            Self::Middle => Color32::from_rgb(0xe6, 0xdb, 0x74),
            Self::End => Color32::from_rgb(0xf9, 0x26, 0x72),
        }
    }
}

/// Each of the three parts occupies one third of the note.
fn edge(note: egui::Rect) -> f32 {
    note.width() / 3.0
}

/// Part of `note` under `x`, or `None` when `x` is outside the note horizontally.
pub(crate) fn part_at(note: egui::Rect, x: f32) -> Option<Part> {
    if !note.x_range().contains(x) {
        return None;
    }
    let edge = edge(note);
    Some(if x < note.left() + edge {
        Part::Start
    } else if x > note.right() - edge {
        Part::End
    } else {
        Part::Middle
    })
}

pub(crate) fn part_rect(note: egui::Rect, part: Part) -> egui::Rect {
    let edge = edge(note);
    let (left, right) = match part {
        Part::Start => (note.left(), note.left() + edge),
        Part::Middle => (note.left() + edge, note.right() - edge),
        Part::End => (note.right() - edge, note.right()),
    };
    egui::Rect::from_x_y_ranges(left..=right, note.y_range())
}

/// The note under `pos` and the part of it, preferring the note drawn last.
pub(crate) fn hovered(notes: &[(usize, egui::Rect)], pos: egui::Pos2) -> Option<(usize, Part)> {
    notes.iter().enumerate().rev().find_map(|(i, &(_, note))| {
        if note.y_range().contains(pos.y) {
            part_at(note, pos.x).map(|part| (i, part))
        } else {
            None
        }
    })
}

/// Whether the blink is in its hover-colored half at `time` seconds.
pub(crate) fn lit(time: f64) -> bool {
    time.rem_euclid(BLINK_PERIOD) < BLINK_PERIOD / 2.0
}
