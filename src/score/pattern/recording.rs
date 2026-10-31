//! Keyboard recording, committed as ordinary pattern note events.
use super::{operations, Catalog, Meter};
use crate::{
    edit,
    model::{Event, Music},
};
use std::time::Duration;

mod pitches;
mod recorder;
mod settings;
pub use pitches::{pitches, Pad, ROW_KEYS};
pub use recorder::Recorder;
pub use settings::{Key, Layout, Mode, Range, Scale, Settings, Start};

/// One new 4/4 pattern of `count` bars holding `events` at pattern-relative ticks.
fn write(
    music: &mut Music,
    source: usize,
    count: u64,
    mut events: Vec<Event>,
) -> Result<u64, String> {
    let measure = u64::from(music.division) * 4;
    if count > 100_000 {
        return Err("Recording exceeds the pattern measure limit".into());
    }
    let id = operations::create(music, source, None)?;
    operations::resize(
        music,
        id,
        vec![
            Meter {
                numerator: 4,
                exponent: 2,
                ticks: measure
            };
            count as usize
        ],
    )?;
    let p = Catalog::read(music)?.patterns[&id].clone();
    // Stable sorting retains Note Off / retrigger order at equal ticks.
    events.sort_by_key(|e| e.tick);
    for mut event in events {
        event.tick = p
            .tick
            .checked_add(event.tick)
            .ok_or("Recording storage overflow")?;
        edit::insert(&mut music.tracks[source], event);
    }
    edit::refresh_deltas(&mut music.tracks[source])?;
    Catalog::read(music)?;
    Ok(id)
}

/// Ticks from `origin` to `at`, rounded; `None` before the origin.
fn ticks(division: u16, tempo: u32, origin: Duration, at: Duration) -> Result<Option<u64>, String> {
    let Some(nanos) = at.checked_sub(origin).map(|d| d.as_nanos()) else {
        return Ok(None);
    };
    let per_quarter = u128::from(tempo) * 1000;
    u64::try_from((nanos * u128::from(division) + per_quarter / 2) / per_quarter)
        .map(Some)
        .map_err(|_| "Recording time overflow".into())
}

fn note(tick: u64, on: bool, pitch: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: if on { 0x90 } else { 0x80 },
        meta_type: 0,
        data: vec![pitch, if on { 80 } else { 0 }],
    }
}

#[cfg(test)]
mod tests;
