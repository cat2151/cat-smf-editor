//! Validated, transient note expansion. The saved document is never modified.
use crate::{
    model::{Event, Music},
    score::{
        musical_time::{MeterMap, MusicalTime, Position},
        note_view::{note_on, NoteView},
    },
};
use std::borrow::Cow;
mod insert;
pub use insert::insert;

pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0same-measure\0";

pub fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(PREFIX)
}

pub fn reference(event: &Event) -> Result<Option<u64>, String> {
    if !owned(event) {
        return Ok(None);
    }
    let payload = &event.data[PREFIX.len()..];
    let measure = match payload {
        [1, bytes @ ..] if bytes.len() == 8 => u64::from_le_bytes(bytes.try_into().unwrap()),
        _ => return Err("Invalid same measure payload".into()),
    };
    if measure == 0 {
        return Err("Same measure number must be positive".into());
    }
    Ok(Some(measure))
}

pub fn event(tick: u64, measure: u64) -> Event {
    let mut data = PREFIX.to_vec();
    data.push(1);
    data.extend(measure.to_le_bytes());
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data,
    }
}

pub fn bounds(map: &MeterMap, measure: u128) -> Result<(u64, u64), String> {
    if measure == 0 {
        return Err("Measure number must be positive".into());
    }
    let mut p = Position {
        measure,
        beat: 1,
        tick_numerator: 0,
        tick_denominator: 1,
    };
    let start = map.tick_at(&p).ok_or("Invalid measure start")?;
    p.measure = p.measure.checked_add(1).ok_or("Measure overflow")?;
    let end = map
        .tick_at(&p)
        .filter(|&end| end > start)
        .ok_or("Invalid measure end")?;
    Ok((start, end))
}

/// Notes wholly contained in a measure; carried, dangling or crossing notes fail.
pub fn notes(events: &[Event], range: (u64, u64)) -> Result<Vec<Event>, String> {
    Notes::new(events).within(range)
}

/// A track's Note Ons, indexed once so each measure is a binary search.
struct Notes<'a> {
    events: &'a [Event],
    /// `(tick, event index, end tick)`, stably sorted by tick; a missing end never ends.
    ons: Vec<(u64, usize, Option<u64>)>,
    /// `reach[k]`: the latest end among `ons[..k]`, `u128::MAX` for a missing end.
    reach: Vec<u128>,
}

impl<'a> Notes<'a> {
    fn new(events: &'a [Event]) -> Self {
        let durations = NoteView::new(events).durations;
        let mut ons: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| note_on(e))
            .map(|(i, on)| {
                (
                    on.tick,
                    i,
                    durations[i].and_then(|d| on.tick.checked_add(d)),
                )
            })
            .collect();
        ons.sort_by_key(|on| on.0);
        let mut reach = Vec::with_capacity(ons.len() + 1);
        reach.push(0);
        for on in &ons {
            let until = on.2.map_or(u128::MAX, u128::from);
            reach.push(until.max(*reach.last().unwrap()));
        }
        Self { events, ons, reach }
    }

    fn within(&self, (start, end): (u64, u64)) -> Result<Vec<Event>, String> {
        let before = self.ons.partition_point(|on| on.0 < start.min(end));
        if self.reach[before] > u128::from(start) {
            return Err("Note crosses measure boundary".into());
        }
        let first = self.ons.partition_point(|on| on.0 < start);
        let last = self.ons.partition_point(|on| on.0 < end).max(first);
        let mut inside = self.ons[first..last].to_vec();
        inside.sort_unstable_by_key(|on| on.1);
        let mut result = Vec::with_capacity(inside.len() * 2);
        for (_, i, until) in inside {
            let on = &self.events[i];
            let until = until
                .filter(|&tick| tick <= end)
                .ok_or("Note crosses measure boundary")?;
            result.push(on.clone());
            result.push(Event {
                tick: until,
                delta: 0,
                status: 0x80 | (on.status & 15),
                meta_type: 0,
                data: vec![on.data[0], 0],
            });
        }
        // Pairing notes emits each off immediately after its on. Chords and
        // overlapping notes must return to tick order before projection or playback.
        // Stable sorting retains on/off order for zero-length and retriggered notes.
        result.sort_by_key(|e| e.tick);
        Ok(result)
    }
}

pub fn expand(music: &Music, track: usize) -> Result<(Cow<'_, [Event]>, u64), String> {
    let source = &music.tracks.get(track).ok_or("Missing track")?.events;
    let mut end = source.last().map_or(0, |e| e.tick);
    if !source.iter().any(owned) {
        return Ok((Cow::Borrowed(source), end));
    }
    let time = MusicalTime::new(music);
    let map = time.map(track).ok_or("Same measure requires PPQN timing")?;
    let notes = Notes::new(source);
    let mut references: Vec<u64> = source.iter().filter(|e| owned(e)).map(|e| e.tick).collect();
    references.sort_unstable();
    let references_in = |(start, end): (u64, u64)| {
        references.partition_point(|&t| t < end) - references.partition_point(|&t| t < start)
    };
    let mut result = source.clone();
    for meta in source.iter().filter(|e| owned(e)) {
        let mut operation = || -> Result<Vec<Event>, String> {
            let from = u128::from(reference(meta)?.unwrap());
            let to = map.position(meta.tick).measure;
            let dest = bounds(map, to)?;
            let origin = bounds(map, from)?;
            if meta.tick != dest.0 {
                return Err("Reference must be at measure start".into());
            }
            if from == to {
                return Err("Self reference".into());
            }
            if dest.1 - dest.0 != origin.1 - origin.0 {
                return Err("Measure lengths differ".into());
            }
            if references_in(origin) != 0 {
                return Err("Chained same measure reference".into());
            }
            if references_in(dest) != 1 {
                return Err("Multiple references in destination".into());
            }
            if !notes.within(dest)?.is_empty() {
                return Err("Destination contains notes".into());
            }
            let mut copied = notes.within(origin)?;
            for e in &mut copied {
                e.tick = dest.0 + (e.tick - origin.0);
            }
            end = end.max(dest.1);
            Ok(copied)
        };
        result.extend(
            operation()
                .map_err(|error| format!("Track {}, tick {}: {error}", track + 1, meta.tick))?,
        );
    }
    result.sort_by_key(|e| e.tick);
    Ok((Cow::Owned(result), end))
}

pub fn validate(music: &Music) -> Result<(), String> {
    super::pattern::Catalog::read(music)?;
    for track in 0..music.tracks.len() {
        expand(music, track)?;
    }
    Ok(())
}

#[cfg(test)]
mod speed_tests;
#[cfg(test)]
mod tests;
