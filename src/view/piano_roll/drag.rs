//! Dragging one part of a note: the rectangle follows the pointer, and the edit is applied on release.
use super::hover::Part;
use cat_smf_editor::{
    edit::{note_length::set_duration, note_move::move_note, note_start::move_start},
    model::Track,
};
use eframe::egui;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Drag {
    /// Index into `Layout::notes`.
    pub(crate) note: usize,
    pub(crate) part: Part,
    /// x where the button went down, in roll coordinates.
    grab: f32,
}

impl Drag {
    pub(crate) fn new(note: usize, part: Part, grab: f32) -> Self {
        Self { note, part, grab }
    }

    /// Ticks from the grab to `x`, rounded to the nearest tick.
    pub(crate) fn shift(&self, x: f32, scale: f32) -> i64 {
        ((x - self.grab) / scale).round() as i64
    }

    /// `note` with its dragged part moved by `shift` ticks.
    pub(crate) fn moved(&self, note: egui::Rect, shift: i64, scale: f32) -> egui::Rect {
        let dx = shift as f32 * scale;
        let (left, right) = match self.part {
            Part::Start => (note.left() + dx, note.right()),
            Part::Middle => (note.left() + dx, note.right() + dx),
            Part::End => (note.left(), note.right() + dx),
        };
        egui::Rect::from_x_y_ranges(left..=right, note.y_range())
    }
}

/// Applies a drag of `part` by `shift` ticks to the note whose Note On is at `on`
/// and whose length is `duration`.
pub(crate) fn apply(
    track: &mut Track,
    on: usize,
    duration: u64,
    part: Part,
    shift: i64,
) -> Result<(), String> {
    match part {
        Part::Start => {
            let tick = track.events[on]
                .tick
                .checked_add_signed(shift)
                .ok_or("The start cannot move before tick 0")?;
            move_start(track, on, tick)
        }
        Part::Middle => move_note(track, on, shift),
        Part::End => {
            let duration = duration
                .checked_add_signed(shift)
                .ok_or("The duration must be at least 1 tick")?;
            set_duration(track, on, duration)
        }
    }
}
