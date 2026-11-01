//! Moves one Note Off; its Note On and every other event keep their ticks.
use crate::message;
use crate::{
    edit::relocate::{pairs, relocate},
    model::Track,
    score::note_view::note_on,
};

/// Gives the Note On at `on` a length of `duration` ticks.
pub fn set_duration(track: &mut Track, on: usize, duration: u64) -> Result<(), String> {
    let events = &track.events;
    if !events.get(on).is_some_and(note_on) {
        return Err(message::NOT_A_NOTE_ON.into());
    }
    if duration == 0 {
        return Err(message::THE_DURATION_MUST_BE_AT_LEAST_1_TICK.into());
    }
    let off = pairs(events.iter())[on].ok_or(message::THIS_NOTE_HAS_NO_NOTE_OFF)?;
    let tick = events[on]
        .tick
        .checked_add(duration)
        .ok_or("長さが大きすぎます")?;
    relocate(track, off, tick)
}

#[cfg(test)]
mod tests;
