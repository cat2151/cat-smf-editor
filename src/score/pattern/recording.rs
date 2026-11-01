//! Keyboard recording, committed as ordinary pattern note events.
use super::{operations, Catalog, Meter};
use crate::message;
use crate::{
    edit,
    model::{Event, Music},
};
use std::time::Duration;

pub mod capture;
mod pitches;
mod recorder;
mod settings;
pub use pitches::{pitches, Pad, ROW_KEYS};
pub use recorder::Recorder;
pub use settings::{Key, Layout, Mode, Range, Scale, Settings, Start};

/// One new 4/4 pattern of `count` bars holding `events` at pattern-relative ticks.
fn write(music: &mut Music, source: usize, count: u64, events: Vec<Event>) -> Result<u64, String> {
    let measure = u64::from(music.division) * 4;
    if count > 100_000 {
        return Err(message::RECORDING_EXCEEDS_THE_PATTERN_MEASURE_LIMIT.into());
    }
    let id = operations::create(music, source, None)?;
    let bar = Meter {
        numerator: 4,
        exponent: 2,
        ticks: measure,
    };
    fill(music, id, vec![bar; count as usize], 0, events)?;
    Ok(id)
}

/// Gives pattern `id` the `meters` and adds `events` at ticks from `offset` into it.
fn fill(
    music: &mut Music,
    id: u64,
    meters: Vec<Meter>,
    offset: u64,
    mut events: Vec<Event>,
) -> Result<(), String> {
    operations::resize(music, id, meters)?;
    let p = Catalog::read(music)?.patterns[&id].clone();
    // Stable sorting retains Note Off / retrigger order at equal ticks.
    events.sort_by_key(|e| e.tick);
    for mut event in events {
        event.tick = p
            .tick
            .checked_add(offset)
            .and_then(|t| t.checked_add(event.tick))
            .ok_or("録音の保存領域が上限を超えました")?;
        edit::insert(&mut music.tracks[p.track], event);
    }
    edit::refresh_deltas(&mut music.tracks[p.track])?;
    Catalog::read(music)?;
    Ok(())
}

/// Ticks from `origin` to `at`, rounded; `None` before the origin.
fn ticks(division: u16, tempo: u32, origin: Duration, at: Duration) -> Result<Option<u64>, String> {
    let Some(nanos) = at.checked_sub(origin).map(|d| d.as_nanos()) else {
        return Ok(None);
    };
    let per_quarter = u128::from(tempo) * 1000;
    u64::try_from((nanos * u128::from(division) + per_quarter / 2) / per_quarter)
        .map(Some)
        .map_err(|_| message::RECORDING_TIME_OVERFLOW.into())
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
