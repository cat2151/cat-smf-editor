//! Keyboard recording, committed as ordinary pattern note events.
use super::{operations, Meter};
use crate::message;
use crate::model::{Event, Music};
use operations::fill;
use std::time::Duration;

pub mod capture;
mod expression;
mod pitches;
mod quantize;
mod recorder;
mod settings;
pub use expression::{Modulation, Velocity};
pub use pitches::{pitches, Pad, ROW_KEYS};
pub use recorder::Recorder;
pub use settings::{Key, Mode, Range, Scale, Settings, Voicing};

/// One new 4/4 pattern of `count` bars holding `events` at pattern-relative ticks.
fn write(music: &mut Music, source: usize, count: u64, events: Vec<Event>) -> Result<u64, String> {
    let measure = u64::from(music.division) * 4;
    if count > 100_000 {
        return Err(message::RECORDING_EXCEEDS_THE_PATTERN_MEASURE_LIMIT.into());
    }
    if events.iter().any(|e| e.status == 0xb0) {
        cc1_baseline(music, source)?;
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

/// The Loop pattern of the container on `source` becomes one 4/4 bar of the phrase loop's
/// `notes` (offsets from the bar start, pitch and velocity, 0 for Note Off) and CC1 `controls`
/// at `tempo` microseconds per quarter; a `None` source creates the container. With `keep`,
/// the Loop pattern's notes of the pitches `notes` does not start stay. With `quantize`, the
/// new notes start and end on sixteenths. Returns the Loop pattern.
pub fn repeat(
    music: &mut Music,
    source: Option<usize>,
    tempo: u32,
    notes: &[(Duration, u8, u8)],
    controls: &[(Duration, u8)],
    keep: bool,
    quantize: bool,
) -> Result<u64, String> {
    if music.division == 0 || music.division & 0x8000 != 0 {
        return Err(message::RECORDING_REQUIRES_PPQN_TIMING.into());
    }
    if tempo == 0 {
        return Err(message::RECORDING_REQUIRES_A_POSITIVE_TEMPO.into());
    }
    if notes.is_empty() {
        return Err(message::RECORDING_IS_EMPTY.into());
    }
    let mut started = [0u64; 128];
    let mut events = vec![];
    for &(offset, pitch, velocity) in notes {
        let tick = ticks(music.division, tempo, Duration::ZERO, offset)?.unwrap_or(0);
        let p = usize::from(pitch);
        let on = velocity > 0;
        // Rounding may put a short note's Note Off on its Note On.
        let tick = if on { tick } else { tick.max(started[p] + 1) };
        if on {
            started[p] = tick;
        }
        events.push(note(tick, pitch, velocity));
    }
    let measure = u64::from(music.division) * 4;
    if quantize {
        let grid = crate::edit::quantize::sixteenth(music.division)?;
        events = self::quantize::events(events, grid, Some(measure));
    }
    for &(offset, value) in controls {
        let tick = ticks(music.division, tempo, Duration::ZERO, offset)?.unwrap_or(0);
        if tick < measure {
            control(&mut events, tick, value);
        }
    }
    let source = match source {
        Some(track) => track,
        None => operations::create_source(music)?,
    };
    if events.iter().any(|e| e.status == 0xb0) {
        cc1_baseline(music, source)?;
    }
    let bar = Meter {
        numerator: 4,
        exponent: 2,
        ticks: measure,
    };
    let existing = super::Catalog::read(music)?
        .patterns
        .into_values()
        .find(|p| p.track == source && p.kind == super::Kind::Loop);
    let p = match existing {
        Some(p) => p,
        None => {
            let id = operations::create_as(music, source, super::Kind::Loop)?;
            super::Catalog::read(music)?.patterns.remove(&id).unwrap()
        }
    };
    if keep {
        use crate::score::note_view::{note_off, note_on};
        let mut played = [false; 128];
        for &(_, pitch, velocity) in notes.iter().filter(|n| n.2 > 0) {
            played[usize::from(pitch & 0x7f)] = velocity > 0;
        }
        // As the loop plays it: notes started in the first bar, cut at its end.
        let mut kept = [0u32; 2048];
        for mut e in super::body::extract(&music.tracks[p.track].events, p.range()?)? {
            let pitch = e.data[0] & 0x7f;
            let key = usize::from(e.status & 15) * 128 + usize::from(pitch);
            e.tick = (e.tick - p.tick).min(measure);
            if played[usize::from(pitch)] {
                continue;
            }
            if note_on(&e) && e.tick < measure {
                kept[key] += 1;
                events.push(e);
            } else if note_off(&e) && kept[key] > 0 {
                kept[key] -= 1;
                events.push(e);
            }
        }
    }
    operations::refill(music, &p, vec![bar], events)?;
    Ok(p.id)
}

/// The notes of the first bar of pattern `id` as offsets from its start at `tempo`
/// microseconds per quarter, pitch and velocity (0 for Note Off); a note still sounding at
/// the bar's end stops there.
pub fn looped(music: &Music, id: u64, tempo: u32) -> Result<Vec<(Duration, u8, u8)>, String> {
    use crate::score::note_view::{note_off, note_on};
    let p = super::Catalog::read(music)?
        .patterns
        .remove(&id)
        .ok_or(message::MISSING_PATTERN)?;
    let measure = u64::from(music.division) * 4;
    let at = |tick: u64| {
        let nanos = u128::from(tick) * u128::from(tempo) * 1000 / u128::from(music.division);
        Duration::from_nanos(nanos as u64)
    };
    let mut on = [false; 128];
    let mut notes = vec![];
    for e in super::body::extract(&music.tracks[p.track].events, p.range()?)? {
        let tick = e.tick - p.tick;
        let pitch = e.data[0];
        let sounding = &mut on[usize::from(pitch & 0x7f)];
        if note_on(&e) && tick < measure {
            *sounding = true;
            notes.push((at(tick), pitch, e.data[1]));
        } else if note_off(&e) && *sounding && tick <= measure {
            *sounding = false;
            notes.push((at(tick), pitch, 0));
        }
    }
    notes.extend(
        (0..128u8)
            .filter(|&p| on[usize::from(p)])
            .map(|p| (at(measure), p, 0)),
    );
    Ok(notes)
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

/// Note On of `velocity`, or Note Off for 0.
fn note(tick: u64, pitch: u8, velocity: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status: if velocity > 0 { 0x90 } else { 0x80 },
        meta_type: 0,
        data: vec![pitch, velocity],
    }
}

/// The container on `source` gets a CC1 baseline of 0, the value a pattern's CC1 returns to,
/// unless it has one.
fn cc1_baseline(music: &mut Music, source: usize) -> Result<(), String> {
    use super::automation::{self, Baseline};
    let set = automation::baselines(music, source)?.iter().any(|b| {
        matches!(
            b,
            Baseline::Midi {
                channel: 0,
                controller: Some(1),
                ..
            }
        )
    });
    if set {
        return Ok(());
    }
    let container = super::Catalog::read(music)?
        .tracks
        .iter()
        .find_map(|(&id, &track)| (track == source).then_some(id))
        .ok_or(message::MISSING_CONTAINER)?;
    automation::set_midi_baseline(music, container, 0, Some(1), 0)
}

/// Appends CC1, the modulation wheel; a CC1 just appended on the same tick gives way.
fn control(events: &mut Vec<Event>, tick: u64, value: u8) {
    if let Some(last) = events
        .last_mut()
        .filter(|e| e.status == 0xb0 && e.tick == tick)
    {
        last.data[1] = value;
        return;
    }
    events.push(Event {
        tick,
        delta: 0,
        status: 0xb0,
        meta_type: 0,
        data: vec![1, value],
    });
}

#[cfg(test)]
mod tests;
