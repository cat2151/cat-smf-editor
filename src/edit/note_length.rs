//! Moves one Note Off; its Note On and every other event keep their ticks.
use crate::{
    edit::relocate::{pairs, relocate},
    model::Track,
    score::note_view::note_on,
};

/// Gives the Note On at `on` a length of `duration` ticks.
pub fn set_duration(track: &mut Track, on: usize, duration: u64) -> Result<(), String> {
    let events = &track.events;
    if !events.get(on).is_some_and(note_on) {
        return Err("Not a Note On".into());
    }
    if duration == 0 {
        return Err("The duration must be at least 1 tick".into());
    }
    let off = pairs(events.iter())[on].ok_or("This note has no Note Off")?;
    let tick = events[on]
        .tick
        .checked_add(duration)
        .ok_or("The duration is too large")?;
    relocate(track, off, tick)
}

#[cfg(test)]
mod tests;
