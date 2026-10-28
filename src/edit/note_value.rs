use crate::{
    model::Track,
    score::note_view::{note_off, note_on},
};
use std::collections::{HashMap, VecDeque};
pub fn pitch(text: &str) -> Result<u8, String> {
    if let Ok(value) = text.trim().parse::<u8>() {
        return (value < 128)
            .then_some(value)
            .ok_or("Pitch must be 0–127".into());
    }
    (0..128)
        .find(|&pitch| crate::score::note_view::note_name(pitch).eq_ignore_ascii_case(text.trim()))
        .ok_or("Invalid note name".into())
}
pub fn set_pitch(track: &mut Track, on: usize, pitch: u8) -> Result<(), String> {
    if pitch > 127 || !track.events.get(on).is_some_and(note_on) {
        return Err("Invalid note".into());
    }
    let mut pending = HashMap::<(u8, u8), VecDeque<usize>>::new();
    let mut off = None;
    for (i, e) in track.events.iter().enumerate() {
        if note_on(e) {
            pending
                .entry((e.status & 15, e.data[0]))
                .or_default()
                .push_back(i);
        } else if note_off(e)
            && pending
                .get_mut(&(e.status & 15, e.data[0]))
                .and_then(VecDeque::pop_front)
                == Some(on)
        {
            off = Some(i);
            break;
        }
    }
    let off = off.ok_or("Note has no matching end")?;
    track.events[on].data[0] = pitch;
    track.events[off].data[0] = pitch;
    Ok(())
}
pub fn set_velocity(track: &mut Track, on: usize, velocity: u8) -> Result<(), String> {
    if !(1..=127).contains(&velocity) || !track.events.get(on).is_some_and(note_on) {
        return Err("Velocity must be 1–127".into());
    }
    track.events[on].data[1] = velocity;
    Ok(())
}
