//! Supplement chord input without changing plain MML or explicit keys.
use eframe::egui;

pub(super) fn apply(ctx: &egui::Context, buffer: &mut String, key: Option<&str>) -> Option<usize> {
    let key = key?;
    if buffer.to_ascii_lowercase().contains("key:") || !cmrt_chord::parses_as_chord(buffer) {
        return None;
    }
    let prefix = format!("Key:{key} ");
    buffer.insert_str(0, &prefix);
    let offset = prefix.chars().count();
    let mut cursor = buffer.chars().count();
    if let Some(id) = ctx.memory(|memory| memory.focused())
        && let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id)
    {
        if let Some(mut range) = state.cursor.char_range() {
            range.primary.index += offset;
            range.secondary.index += offset;
            cursor = range.primary.index;
            state.cursor.set_char_range(Some(range));
        }
        state.store(ctx, id);
    }
    ctx.request_repaint();
    Some(cursor)
}
