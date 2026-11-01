//! Moves one Note On; its Note Off and every other event keep their ticks.
use crate::message;
use crate::{
    edit::relocate::{pairs, relocate},
    model::Track,
    score::note_view::note_on,
};

pub fn move_start(track: &mut Track, on: usize, tick: u64) -> Result<(), String> {
    let events = &track.events;
    if !events.get(on).is_some_and(note_on) {
        return Err(message::NOT_A_NOTE_ON.into());
    }
    let off = pairs(events.iter())[on].ok_or(message::THIS_NOTE_HAS_NO_NOTE_OFF)?;
    if tick >= events[off].tick {
        return Err("始まりはNote Offより前に置きます".into());
    }
    relocate(track, on, tick)
}

#[cfg(test)]
mod tests;
