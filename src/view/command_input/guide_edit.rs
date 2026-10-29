//! Minimal value and absolute-tick editing through the independent command input owner.
use crate::app::App;
use cat_smf_editor::{
    edit::meta_input::Kind,
    score::{chord_guide, key_signature::KeySignature},
};

pub(crate) fn on_guide(a: &App) -> bool {
    a.patterns.editing.is_none()
        && a.patterns.raw_context.is_none()
        && a.loaded
            .as_ref()
            .is_some_and(|l| chord_guide::track(&l.music) == Some(a.track))
}

pub(crate) fn selected(a: &App) -> Option<usize> {
    if a.note_mode {
        a.cursor_row
            .and_then(
                |row| match a.loaded.as_ref()?.display_rows[a.track].get(row) {
                    cat_smf_editor::score::display_rows::DisplayRow::Event(i) => Some(i),
                    _ => None,
                },
            )
            .or(a.selected_event)
    } else {
        a.cursor_row.or(a.selected_event)
    }
}

pub(super) fn state(a: &App, moving: bool) -> Option<super::insert_event::State> {
    if !on_guide(a) {
        return None;
    }
    let index = selected(a)?;
    let event = a.loaded.as_ref()?.music.tracks[a.track].events.get(index)?;
    let (kind, text) = if let Some(text) = chord_guide::chord(event) {
        (Kind::Chord, text.to_owned())
    } else {
        (Kind::KeySignature, KeySignature::from_event(event)?.name())
    };
    Some(super::insert_event::State {
        kind: Some(kind),
        edit: Some(index),
        move_event: moving,
        query: if moving { event.tick.to_string() } else { text },
        ..Default::default()
    })
}

pub(super) fn apply(
    a: &mut App,
    index: usize,
    kind: Kind,
    text: &str,
    moving: bool,
) -> Result<(), String> {
    if !on_guide(a) {
        return Err("Select the chord guide track".into());
    }
    let before = a.loaded.as_ref().unwrap().music.clone();
    let mut music = before.clone();
    let track = &mut music.tracks[a.track];
    let old = track.events.get(index).ok_or("Invalid event")?;
    let event = if moving {
        let mut event = old.clone();
        event.tick = text.trim().parse().map_err(|_| "Invalid tick")?;
        event
    } else {
        kind.event(old.tick, text)?
    };
    track.events.remove(index);
    // Value changes preserve same-tick ordering; moves use the normal insertion order.
    let at = if moving {
        let at = track
            .events
            .partition_point(|e| e.tick <= event.tick && e.meta_type != 0x2f);
        cat_smf_editor::edit::insert(track, event);
        at
    } else {
        track.events.insert(index, event);
        index
    };
    cat_smf_editor::edit::refresh_deltas(track)?;
    a.apply_music(music)?;
    a.undo.push(before);
    a.selected_event = Some(at);
    a.track_view.cursor_tick = None;
    a.cursor_position = None;
    crate::cursor::moves::sync_mode(a);
    a.reveal_cursor = true;
    Ok(())
}

pub(crate) fn delete(a: &mut App) -> Result<(), String> {
    if !on_guide(a) {
        return Err("Select the chord guide track".into());
    }
    let index = selected(a).ok_or("Select a guide event")?;
    let before = a.loaded.as_ref().unwrap().music.clone();
    let mut music = before.clone();
    let track = &mut music.tracks[a.track];
    let event = track.events.get(index).ok_or("Invalid event")?;
    if chord_guide::chord(event).is_none() && KeySignature::from_event(event).is_none() {
        return Err("Select a key or chord".into());
    }
    track.events.remove(index);
    cat_smf_editor::edit::refresh_deltas(track)?;
    a.apply_music(music)?;
    a.undo.push(before);
    Ok(())
}

#[cfg(test)]
mod tests;
