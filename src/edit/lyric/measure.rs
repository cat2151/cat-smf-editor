//! Lyrics of one range of ticks, typed again: one mora per note tick in the range, from its start.
//! Moras without notes get notes on note number 60.
use super::lyric;
use crate::{
    model::{Event, Music, Track},
    score::note_view::{note_on, NoteView},
};

/// Small kana that join the previous character into one mora.
const SMALL: &str = "ゃゅょぁぃぅぇぉゎャュョァィゥェォヮ";

/// The added notes of a range without notes: eighth notes at this velocity.
const VELOCITY: u8 = 100;

/// One mora per character, except that small kana join the one before. Whitespace only separates.
pub fn moras(text: &str) -> Vec<String> {
    let mut moras: Vec<String> = Vec::new();
    let mut joinable = false;
    for c in text.chars() {
        if c.is_whitespace() {
            joinable = false;
            continue;
        }
        match moras.last_mut() {
            Some(last) if joinable && SMALL.contains(c) => last.push(c),
            _ => moras.push(c.to_string()),
        }
        joinable = true;
    }
    moras
}

/// Gives the moras of `text` to the note ticks in `range` (start inclusive, end exclusive) in order,
/// overwriting their lyrics; a chord's tick counts once. Notes past the last mora keep their lyrics.
/// Moras past the last note get notes on note number 60, one after another from the end of the last
/// note, each like the note before it; in a range without notes, eighth notes from its start.
/// A range whose only note starts it first halves that note until every mora fits.
/// Moras whose note would start at or after the end of `range` are dropped.
/// Lyrics in `range` at a tick without a Note On are removed.
pub fn replace(
    music: &mut Music,
    track: usize,
    range: (u64, u64),
    text: &str,
) -> Result<(), String> {
    let division = u64::from(music.division);
    let target = music.tracks.get_mut(track).ok_or("No such track")?;
    let inside = |tick: u64| range.0 <= tick && tick < range.1;
    let mut ticks: Vec<u64> = target
        .events
        .iter()
        .filter(|e| note_on(e) && inside(e.tick))
        .map(|e| e.tick)
        .collect();
    ticks.dedup();
    target
        .events
        .retain(|e| !(lyric(e) && inside(e.tick) && ticks.binary_search(&e.tick).is_err()));
    let moras = moras(text);
    shorten_lone_note(target, range, moras.len())?;
    if moras.len() > ticks.len() {
        let eighth = division / 2;
        let (mut tick, duration, channel, velocity) = match last_note(target, ticks.last()) {
            Some(note) if note.duration > 0 => (
                note.tick + note.duration,
                note.duration,
                note.channel,
                note.velocity,
            ),
            Some(note) => (note.tick + eighth, eighth, note.channel, note.velocity),
            None => (
                range.0,
                eighth,
                super::super::mml::channel(target, None),
                VELOCITY,
            ),
        };
        while ticks.len() < moras.len() && inside(tick) {
            add(target, tick, duration, channel, velocity);
            ticks.push(tick);
            tick += duration;
        }
    }
    for (&tick, mora) in ticks.iter().zip(&moras) {
        let on = target
            .events
            .iter()
            .position(|e| e.tick == tick && note_on(e))
            .ok_or("Not a Note On")?;
        super::set(target, on, mora)?;
    }
    super::super::refresh_deltas(target)
}

/// Halves the only note of `range` while it starts the range and `count` notes of its length overflow it.
fn shorten_lone_note(track: &mut Track, range: (u64, u64), count: usize) -> Result<(), String> {
    let mut ons = (0..track.events.len()).filter(|&i| {
        note_on(&track.events[i]) && (range.0..range.1).contains(&track.events[i].tick)
    });
    let (Some(on), None) = (ons.next(), ons.next()) else {
        return Ok(());
    };
    let Some(note) = last_note(track, Some(&range.0)) else {
        return Ok(());
    };
    let mut duration = note.duration;
    while duration % 2 == 0 && duration > 0 && (range.1 - range.0) / duration < count as u64 {
        duration /= 2;
    }
    if duration == note.duration {
        return Ok(());
    }
    crate::edit::note_length::set_duration(track, on, duration)
}

struct Note {
    tick: u64,
    duration: u64,
    channel: u8,
    velocity: u8,
}

/// The note ending last among the Note Ons at `tick`.
fn last_note(track: &Track, tick: Option<&u64>) -> Option<Note> {
    let tick = *tick?;
    let view = NoteView::new(&track.events);
    (0..track.events.len())
        .filter(|&i| track.events[i].tick == tick && note_on(&track.events[i]))
        .map(|i| Note {
            tick,
            duration: view.durations[i].unwrap_or(0),
            channel: track.events[i].status & 15,
            velocity: track.events[i].data[1],
        })
        .max_by_key(|note| note.duration)
}

/// A note on note number 60.
fn add(track: &mut Track, tick: u64, duration: u64, channel: u8, velocity: u8) {
    let event = |tick, status, velocity| Event {
        delta: 0,
        tick,
        status,
        meta_type: 0,
        data: vec![60, velocity],
    };
    super::super::insert(track, event(tick, 0x90 | channel, velocity));
    super::super::insert(track, event(tick + duration, 0x80 | channel, 0));
}

#[cfg(test)]
mod tests;
