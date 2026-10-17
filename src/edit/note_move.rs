//! Moves one note's Note On and Note Off by the same amount; every other event keeps its tick.
use crate::{
    edit::relocate::{pairs, relocate},
    model::Track,
    score::note_view::note_on,
};

/// Shifts the note at `on` by `shift` ticks, keeping its duration.
/// On error the track is unchanged.
pub fn move_note(track: &mut Track, on: usize, shift: i64) -> Result<(), String> {
    let events = &track.events;
    if !events.get(on).is_some_and(note_on) {
        return Err("Not a Note On".into());
    }
    let off = pairs(events.iter())[on].ok_or("This note has no Note Off")?;
    let shifted = |tick: u64| {
        tick.checked_add_signed(shift)
            .ok_or("The note cannot move before tick 0 or that far")
    };
    let start = shifted(events[on].tick)?;
    let end = shifted(events[off].tick)?;
    // Moving the leading event first keeps the Note On before its Note Off at every step,
    // so neither event passes the other and both indices stay valid.
    let mut moved = track.clone();
    if shift > 0 {
        relocate(&mut moved, off, end)?;
        relocate(&mut moved, on, start)?;
    } else {
        relocate(&mut moved, on, start)?;
        relocate(&mut moved, off, end)?;
    }
    *track = moved;
    Ok(())
}

#[cfg(test)]
mod tests;
