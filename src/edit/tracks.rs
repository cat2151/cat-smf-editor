//! Minimal new songs and intersection-based track initialization.
use crate::model::{Event, Music, Track};
use crate::score::note_view::note_on;

fn event(tick: u64, status: u8, meta_type: u8, data: &[u8]) -> Event {
    Event {
        delta: 0,
        tick,
        status,
        meta_type,
        data: data.to_vec(),
    }
}

pub fn new_song() -> Music {
    let mut track = Track {
        events: vec![
            event(0, 0xff, 0x51, &[7, 0xa1, 0x20]),
            event(0, 0xff, 0x58, &[4, 2, 24, 8]),
        ],
    };
    append_note(&mut track, 480).unwrap();
    Music {
        format: 1,
        division: 480,
        tracks: vec![track],
    }
}

fn same(a: &Event, b: &Event) -> bool {
    (a.status, a.meta_type, &a.data) == (b.status, b.meta_type, &b.data)
}

fn initial(track: &Track) -> Vec<Event> {
    let tick = track.events.first().map_or(0, |e| e.tick);
    track
        .events
        .iter()
        .take_while(|e| e.tick == tick)
        .filter(|e| {
            !matches!(e.status & 0xf0, 0x80 | 0x90) && !(e.status == 0xff && e.meta_type == 0x2f)
        })
        .cloned()
        .collect()
}

/// Common initial non-note events, matched as whole payloads rather than fields.
/// Timing is normalized; source order and the common duplicate count are retained.
pub(super) fn common_initial<'a>(tracks: impl Iterator<Item = &'a Track>) -> Vec<Event> {
    let mut sources = tracks.map(initial);
    let mut init = sources.next().unwrap_or_default();
    for mut source in sources {
        init.retain(|e| {
            if let Some(i) = source.iter().position(|other| same(e, other)) {
                source.remove(i);
                true
            } else {
                false
            }
        });
    }
    for e in &mut init {
        e.tick = 0;
        e.delta = 0;
    }
    init
}

pub fn add(music: &mut Music) -> Result<(), String> {
    if music.tracks.len() >= 32767 {
        return Err("Too many SMF tracks".into());
    }
    let sources: Vec<_> = music
        .tracks
        .iter()
        .filter(|t| t.events.iter().any(note_on))
        .collect();
    let init = if sources.len() < 2 {
        Vec::new()
    } else {
        common_initial(sources.into_iter())
    };
    let mut track = Track { events: init };
    let duration = if music.division & 0x8000 == 0 {
        u64::from(music.division)
    } else {
        // SMPTE ticks have fixed seconds; a quarter follows the initial tempo.
        let tempo = if music.format == 2 {
            track.events.iter().collect::<Vec<_>>()
        } else {
            music.tracks.iter().flat_map(|t| &t.events).collect()
        }
        .into_iter()
        .filter(|e| e.tick == 0 && e.status == 0xff && e.meta_type == 0x51 && e.data.len() == 3)
        .map(|e| (u32::from(e.data[0]) << 16) | (u32::from(e.data[1]) << 8) | u32::from(e.data[2]))
        .rfind(|&t| t > 0)
        .unwrap_or(500_000);
        crate::score::audition_timing::TempoMap::new(music, 0)
            .tick_after(0, f64::from(tempo) / 1_000_000.0)
            .max(1)
    };
    if duration == 0 {
        return Err("Invalid ticks per quarter note".into());
    }
    append_note(&mut track, duration)?;
    let mut next = music.clone();
    if next.format == 0 {
        next.format = 1;
    }
    next.tracks.push(track);
    crate::smf_io::encode(&next)?;
    *music = next;
    Ok(())
}

fn append_note(track: &mut Track, duration: u64) -> Result<(), String> {
    let channel = crate::edit::mml::channel(track, None);
    track.events.extend([
        event(0, 0x90 | channel, 0, &[60, 100]),
        event(duration, 0x80 | channel, 0, &[60, 0]),
        event(duration, 0xff, 0x2f, &[]),
    ]);
    super::refresh_deltas(track)
}

#[cfg(test)]
mod tests;
