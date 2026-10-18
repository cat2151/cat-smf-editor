//! Editor-specific projection of generated SMF onto one track/channel (ADR 0010).
use crate::{
    model::{Event, Music, Track},
    score::note_view::note_on,
};

/// Prefer the selected channel, then an existing note channel, then any channel
/// event. Empty/meta-only tracks use MIDI channel 1.
pub fn channel(track: &Track, selected: Option<usize>) -> u8 {
    let channel_event = |e: &&Event| (0x80..=0xef).contains(&e.status);
    selected
        .and_then(|i| track.events.get(i))
        .filter(|e| channel_event(e))
        .or_else(|| track.events.iter().find(|e| note_on(e)))
        .or_else(|| track.events.iter().find(channel_event))
        .map_or(0, |e| e.status & 15)
}

/// Returns the tick of the last inserted note event.
pub fn insert(
    music: &mut Music,
    track: usize,
    tick: u64,
    channel: u8,
    input: &str,
) -> Result<u64, String> {
    let (bytes, _) = crate::mml::to_smf(input)?;
    let source = crate::ffi::parse(&bytes)?;
    insert_smf(music, track, tick, channel, &source)
}

pub fn insert_smf(
    music: &mut Music,
    track: usize,
    tick: u64,
    channel: u8,
    source: &Music,
) -> Result<u64, String> {
    if channel > 15 {
        return Err("Invalid MIDI channel".into());
    }
    if music.division == 0
        || source.division == 0
        || music.division & 0x8000 != 0
        || source.division & 0x8000 != 0
    {
        return Err("MML insertion requires ticks per quarter note".into());
    }
    let mut destination = music.tracks.get(track).ok_or("Invalid track")?.clone();
    let mut notes = Vec::new();
    for event in source.tracks.iter().flat_map(|t| &t.events) {
        if !matches!(event.status & 0xf0, 0x80 | 0x90) || event.data.len() != 2 {
            continue;
        }
        let offset = (u128::from(event.tick) * u128::from(music.division)
            + u128::from(source.division) / 2)
            / u128::from(source.division);
        let at = tick
            .checked_add(u64::try_from(offset).map_err(|_| "Tick overflow")?)
            .ok_or("Tick overflow")?;
        let mut note = event.clone();
        note.tick = at;
        note.delta = 0;
        note.status = (if note_on(event) { 0x90 } else { 0x80 }) | channel;
        notes.push(note);
    }
    if !notes.iter().any(note_on) {
        return Err("MMLに発音ノートがありません".into());
    }
    // Preserve source order for simultaneous events, including zero-length notes.
    notes.sort_by_key(|e| e.tick);
    let end = notes.last().map_or(tick, |e| e.tick);
    for note in notes {
        super::insert(&mut destination, note);
    }
    super::refresh_deltas(&mut destination)?;
    let mut next = music.clone();
    next.tracks[track] = destination;
    crate::smf_io::encode(&next)?;
    *music = next;
    Ok(end)
}

#[cfg(test)]
mod tests;
