//! The note view's cells, and the colored pieces of the row's content column.
use crate::view::{pitch_keyboard::note_color, row::Span};
use cat_smf_editor::{
    model::Event,
    score::event_content::{decoded, parts, Part},
    score::key_signature::KeySignature,
    score::note_view::{note_name, note_on, NoteView},
};
use eframe::egui::Color32;

/// Meta contents in Monokai purple; their kinds keep the text color.
pub(super) const VALUE_COLOR: Color32 = Color32::from_rgb(0xae, 0x81, 0xff);

/// Appends the content cells; the returned pieces draw the first of them.
pub(super) fn append(
    cells: &mut Vec<String>,
    event: &Event,
    notes: &NoteView,
    index: usize,
    key: Option<KeySignature>,
) -> Vec<Span> {
    let note = |pitch| (note_name(pitch), Some(note_color(pitch, key)));
    if note_on(event) {
        cells.extend([
            note_name(event.data[0]),
            notes.durations[index].map_or_else(|| "?".into(), |d| d.to_string()),
            if notes.uniform_velocity_head.is_none_or(|head| head == index) {
                event.data[1].to_string()
            } else {
                String::new()
            },
        ]);
        return vec![note(event.data[0])];
    }
    cells.push(decoded(event));
    parts(event)
        .into_iter()
        .map(|part| match part {
            Part::Label(text) => (text, None),
            Part::Value(text) => (text, Some(VALUE_COLOR)),
            Part::Note(pitch) => note(pitch),
        })
        .collect()
}
