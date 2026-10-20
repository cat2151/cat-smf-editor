//! Lyric meta events (`0xFF 0x05`, UTF-8) held per tick: a note's lyric is the one at its tick.
use crate::{
    model::{Event, Track},
    score::note_view::note_on,
};

pub mod measure;

pub const LYRIC: u8 = 0x05;

fn lyric(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == LYRIC
}

/// The first non-empty lyric at `tick`.
pub fn text_at(events: &[Event], tick: u64) -> Option<String> {
    events
        .iter()
        .filter(|e| e.tick == tick && lyric(e))
        .map(|e| String::from_utf8_lossy(&e.data).into_owned())
        .find(|text| !text.is_empty())
}

/// Makes `text` the only lyric at the tick of the Note On at `on`, just before that note.
/// An empty `text` removes the lyrics at that tick.
pub fn set(track: &mut Track, on: usize, text: &str) -> Result<(), String> {
    let note = track
        .events
        .get(on)
        .filter(|e| note_on(e))
        .ok_or("Not a Note On")?;
    let tick = note.tick;
    let removed_before = track.events[..on]
        .iter()
        .filter(|e| e.tick == tick && lyric(e))
        .count();
    track.events.retain(|e| !(e.tick == tick && lyric(e)));
    if !text.is_empty() {
        track.events.insert(
            on - removed_before,
            Event {
                delta: 0,
                tick,
                status: 0xff,
                meta_type: LYRIC,
                data: text.as_bytes().to_vec(),
            },
        );
    }
    super::refresh_deltas(track)
}

#[cfg(test)]
mod tests;
