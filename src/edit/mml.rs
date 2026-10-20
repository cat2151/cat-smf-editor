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

/// Returns the end of the MML phrase, including trailing rests.
pub fn insert(
    music: &mut Music,
    track: usize,
    tick: u64,
    channel: u8,
    input: &str,
) -> Result<u64, String> {
    insert_smf(music, track, tick, channel, &source(input)?)
}

/// Like [`insert`], but the phrase overwrites the track's notes under it, rests
/// included: a note starting inside the phrase is removed, and a note sounding
/// into it from earlier ends where the phrase starts.
pub fn replace(
    music: &mut Music,
    track: usize,
    tick: u64,
    channel: u8,
    input: &str,
) -> Result<u64, String> {
    let source = source(input)?;
    let end = phrase_tick(music, tick, &source)?;
    let mut next = music.clone();
    let destination = next.tracks.get_mut(track).ok_or("Invalid track")?;
    clear(destination, tick, end)?;
    if source.tracks.iter().flat_map(|t| &t.events).any(note_on) {
        insert_smf(&mut next, track, tick, channel, &source)?;
    } else {
        crate::smf_io::encode(&next)?;
    }
    *music = next;
    Ok(end)
}

/// Overwrite exactly one measure, preserving notes and events outside it.
pub fn replace_measure(
    music: &mut Music,
    track: usize,
    range: (u64, u64),
    channel: u8,
    input: &str,
) -> Result<(), String> {
    let source = source(input)?;
    if phrase_tick(music, range.0, &source)? > range.1 {
        return Err("MMLが1小節を超えています".into());
    }
    let mut next = music.clone();
    clear(
        next.tracks.get_mut(track).ok_or("Invalid track")?,
        range.0,
        range.1,
    )?;
    if source.tracks.iter().flat_map(|t| &t.events).any(note_on) {
        insert_smf(&mut next, track, range.0, channel, &source)?;
    }
    *music = next;
    Ok(())
}

fn clear(track: &mut Track, start: u64, end: u64) -> Result<(), String> {
    let pairs = super::relocate::pairs(track.events.iter());
    let mut remove = vec![false; track.events.len()];
    for (on, off) in pairs.iter().enumerate() {
        if note_on(&track.events[on]) && (start..end).contains(&track.events[on].tick) {
            remove[on] = true;
            if let Some(off) = *off {
                remove[off] = true;
            }
        }
    }
    let mut i = 0;
    track.events.retain(|_| {
        let keep = !remove[i];
        i += 1;
        keep
    });
    // Each move reorders events, so pair again after every one.
    while let Some(off) = super::relocate::pairs(track.events.iter())
        .into_iter()
        .enumerate()
        .filter(|&(on, _)| note_on(&track.events[on]) && track.events[on].tick < start)
        .find_map(|(_, off)| off.filter(|&off| track.events[off].tick > start))
    {
        super::relocate::relocate(track, off, start)?;
    }
    super::refresh_deltas(track)
}

fn source(input: &str) -> Result<Music, String> {
    let (bytes, _) = crate::mml::to_smf(input)?;
    let mut source = crate::ffi::parse(&bytes)?;
    // SMF drops trailing rests. Ask the same converter for each channel's final
    // clock using non-sounding program markers; these markers are never inserted.
    let end = phrase_end(input);
    if let Some(e) = source.tracks[0]
        .events
        .last_mut()
        .filter(|e| e.status == 0xff && e.meta_type == 0x2f)
    {
        e.tick = e.tick.max(end);
    }
    Ok(source)
}

/// The tick in `music` where `source` placed at `tick` ends.
fn phrase_tick(music: &Music, tick: u64, source: &Music) -> Result<u64, String> {
    if music.division == 0
        || source.division == 0
        || music.division & 0x8000 != 0
        || source.division & 0x8000 != 0
    {
        return Err("MML insertion requires ticks per quarter note".into());
    }
    let source_end = source
        .tracks
        .iter()
        .flat_map(|t| &t.events)
        .map(|e| e.tick)
        .max()
        .unwrap_or(0);
    let offset = (u128::from(source_end) * u128::from(music.division)
        + u128::from(source.division) / 2)
        / u128::from(source.division);
    tick.checked_add(u64::try_from(offset).map_err(|_| "Tick overflow")?)
        .ok_or_else(|| "Tick overflow".into())
}

fn phrase_end(input: &str) -> u64 {
    use mmlabc_to_smf::{mml_preprocessor, pass1_parser, pass2_ast, pass3_events, types::AstNote};
    let resolved = cmrt_chord::resolve_chord_or_mml(input);
    let preprocessed = mml_preprocessor::extract_embedded_json(&resolved.mml);
    let tokens = pass1_parser::parse_mml(&preprocessed.remaining_mml);
    let mut ast = pass2_ast::tokens_to_ast(&tokens);
    let channels: std::collections::BTreeSet<_> = ast.notes.iter().map(|n| n.channel).collect();
    for channel in channels {
        ast.notes.push(AstNote {
            note_type: "program_change".into(),
            pitch: 0,
            name: String::new(),
            channel,
            chord_id: None,
            length: None,
            dots: None,
            velocity: None,
        });
    }
    pass3_events::ast_to_events(&ast, true)
        .iter()
        .map(|e| u64::from(e.time))
        .max()
        .unwrap_or(0)
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
    let end = phrase_tick(music, tick, source)?;
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
    for note in notes {
        super::insert(&mut destination, note);
    }
    if let Some(e) = destination
        .events
        .last_mut()
        .filter(|e| e.status == 0xff && e.meta_type == 0x2f)
    {
        e.tick = e.tick.max(end);
    }
    if !destination
        .events
        .last()
        .is_some_and(|e| e.status == 0xff && e.meta_type == 0x2f)
    {
        destination.events.push(Event {
            tick: end.max(destination.events.last().map_or(0, |e| e.tick)),
            delta: 0,
            status: 0xff,
            meta_type: 0x2f,
            data: vec![],
        });
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
