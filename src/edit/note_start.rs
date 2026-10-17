//! Moves one Note On; its Note Off and every other event keep their ticks.
use crate::{
    edit::relocate::{pairs, relocate},
    model::Track,
    score::note_view::note_on,
};

pub fn move_start(track: &mut Track, on: usize, tick: u64) -> Result<(), String> {
    let events = &track.events;
    if !events.get(on).is_some_and(note_on) {
        return Err("Not a Note On".into());
    }
    let off = pairs(events.iter())[on].ok_or("This note has no Note Off")?;
    if tick >= events[off].tick {
        return Err("The start must be before the Note Off".into());
    }
    relocate(track, on, tick)
}

#[cfg(test)]
mod tests;
