//! One optional guide on the ordinary Format 1 PPQN timeline.
use crate::model::{Event, Music, Track};

pub const ROLE: &[u8] = b"\x7dcat-smf-editor\0chord-guide\0";
pub const CHORD: &[u8] = b"\x7dcat-smf-editor\0chord\0";

pub fn supported(music: &Music) -> bool {
    music.format == 1 && music.division != 0 && music.division & 0x8000 == 0
}

pub fn is_guide(track: &Track) -> bool {
    track.events.iter().any(|e| {
        e.status == 0xff && e.meta_type == 0x7f && e.data.strip_prefix(ROLE) == Some(&[1][..])
    })
}

pub fn track(music: &Music) -> Option<usize> {
    supported(music)
        .then(|| music.tracks.iter().position(is_guide))
        .flatten()
}

pub fn chord(event: &Event) -> Option<&str> {
    if event.status != 0xff || event.meta_type != 0x7f {
        return None;
    }
    chord_text(&event.data)
}

pub fn chord_text(data: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(data.strip_prefix(CHORD)?.strip_prefix(&[1])?).ok()?;
    (!text.trim().is_empty() && !text.contains(['\r', '\n', '\t'])).then_some(text)
}

pub fn event(tick: u64, text: &str) -> Result<Event, String> {
    if text.trim().is_empty() || text.contains(['\r', '\n', '\t']) {
        return Err("Expected a single-line chord".into());
    }
    Ok(meta(tick, [CHORD, &[1], text.as_bytes()].concat()))
}

fn meta(tick: u64, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data,
    }
}

pub fn add(music: &mut Music) -> Result<(), String> {
    if !supported(music) {
        return Err("Chord guide requires Format 1 PPQN".into());
    }
    if track(music).is_some() {
        return Err("Chord guide already exists".into());
    }
    if music.tracks.len() >= 32767 {
        return Err("Too many SMF tracks".into());
    }
    let mut guide = Track {
        events: vec![meta(0, [ROLE, &[1]].concat())],
    };
    crate::edit::tracks::rename(&mut guide, "Chord guide")?;
    guide.events.push(Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    });
    music.tracks.push(guide);
    Ok(())
}

pub fn chords(music: &Music) -> impl Iterator<Item = &Event> {
    track(music)
        .into_iter()
        .flat_map(|t| &music.tracks[t].events)
        .filter(|e| chord(e).is_some())
}
