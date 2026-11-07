//! Pitched notes on the 16th-note step grid. Operations name a pitch, a measure and a step;
//! how notes are laid out on screen is the caller's concern. Each note keeps its own Note Off.
use crate::{
    edit,
    model::{Event, Music, Track},
    score::{
        drums::Measure,
        musical_time::{MeterMap, MusicalTime},
        note_view::note_on,
    },
};

/// One sounding note: its Note On / Note Off indices in the track and their ticks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub pitch: u8,
    pub on: usize,
    pub off: usize,
    pub start: u64,
    pub end: u64,
    pub velocity: u8,
}
impl Note {
    /// Zero-length notes still occupy the step they start in.
    fn covers(&self, start: u64, end: u64) -> bool {
        self.start < end && self.end.max(self.start + 1) > start
    }
}

/// What one step of one pitch shows. `aligned` is false for notes whose start or end
/// is off the step grid (MML or recorded input).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Head { aligned: bool },
    Continuation { aligned: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    Longer,
    Shorter,
}

/// Notes with a paired Note Off, ordered by pitch then start.
pub fn notes(track: &Track) -> Vec<Note> {
    let pairs = edit::relocate::pairs(track.events.iter());
    let mut notes: Vec<_> = track
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e))
        .filter_map(|(on, e)| {
            let off = pairs[on]?;
            Some(Note {
                pitch: e.data[0],
                on,
                off,
                start: e.tick,
                end: track.events[off].tick,
                velocity: e.data[1],
            })
        })
        .collect();
    notes.sort_by_key(|n| (n.pitch, n.start, n.on));
    notes
}

/// The cells of `pitch` across `measure`. A head wins over a continuation in the same step.
pub fn cells(
    notes: &[Note],
    map: &MeterMap,
    division: u16,
    measure: &Measure,
    pitch: u8,
) -> Vec<Cell> {
    let notes: Vec<_> = notes.iter().filter(|n| n.pitch == pitch).collect();
    (0..measure.steps.len())
        .map(|step| {
            let (start, end) = bounds(measure, step);
            let aligned = |n: &Note| {
                on_grid(map, division, n.start) && on_grid(map, division, n.end.max(n.start + 1))
            };
            if let Some(n) = notes.iter().find(|n| (start..end).contains(&n.start)) {
                Cell::Head {
                    aligned: aligned(n),
                }
            } else if let Some(n) = notes.iter().find(|n| n.covers(start, end)) {
                Cell::Continuation {
                    aligned: aligned(n),
                }
            } else {
                Cell::Empty
            }
        })
        .collect()
}

/// Removes every note of `pitch` covering the step, or places a one-step note there.
pub fn toggle(
    music: &mut Music,
    track: usize,
    pitch: u8,
    number: u64,
    step: usize,
    velocity: u8,
) -> Result<(), String> {
    if pitch > 127 || !(1..=127).contains(&velocity) {
        return Err("Invalid note or velocity".into());
    }
    let measure = measure(music, track, number)?;
    if step >= measure.steps.len() {
        return Err("Invalid step".into());
    }
    let (start, end) = bounds(&measure, step);
    let source = music.tracks.get_mut(track).ok_or("Missing track")?;
    let covering: Vec<_> = notes(source)
        .into_iter()
        .filter(|n| n.pitch == pitch && n.covers(start, end))
        .map(|n| n.on)
        .collect();
    if !covering.is_empty() {
        return edit::event_range::delete(source, &covering, true);
    }
    let channel = edit::mml::channel(source, None);
    place(source, event(start, 0x90 | channel, pitch, velocity));
    place(source, event(end, 0x80 | channel, pitch, 0));
    edit::refresh_deltas(source)
}

/// Moves the end of the note covering the step by one step boundary. Stops without error at
/// the next head of the same pitch, at `limit` (the end of the editable range) and at one step.
/// An end off the grid lands on the nearest boundary in that direction; the start never moves.
pub fn resize(
    music: &mut Music,
    track: usize,
    pitch: u8,
    number: u64,
    step: usize,
    length: Length,
    limit: u64,
) -> Result<(), String> {
    let time = MusicalTime::new(music);
    let map = time.map(track).ok_or("Missing measure map")?;
    let division = music.division;
    let measure = Measure::new(map, division, number)?;
    if step >= measure.steps.len() {
        return Err("Invalid step".into());
    }
    let (start, end) = bounds(&measure, step);
    let source = music.tracks.get_mut(track).ok_or("Missing track")?;
    let before = notes(source);
    let Some(note) = before
        .iter()
        .find(|n| n.pitch == pitch && n.covers(start, end))
        .cloned()
    else {
        return Ok(());
    };
    let target = match length {
        Length::Longer => {
            let target = next(map, division, note.end)?;
            let blocked = before
                .iter()
                .any(|n| n.pitch == pitch && n.start > note.start && n.start < target);
            if target > limit || blocked {
                return Ok(());
            }
            target
        }
        Length::Shorter => {
            let Some(target) = previous(map, division, note.end)? else {
                return Ok(());
            };
            if target < next(map, division, note.start)? {
                return Ok(());
            }
            target
        }
    };
    let mut edited = source.clone();
    let mut off = edited.events.remove(note.off);
    off.tick = target;
    place(&mut edited, off);
    edit::refresh_deltas(&mut edited)?;
    let shape = |list: &[Note]| {
        let mut v: Vec<_> = list
            .iter()
            .map(|n| (n.pitch, n.start, n.end, n.velocity))
            .collect();
        v.sort();
        v
    };
    let expected: Vec<_> = before
        .iter()
        .map(|n| {
            if n.on == note.on {
                Note {
                    end: target,
                    ..n.clone()
                }
            } else {
                n.clone()
            }
        })
        .collect();
    // Off-grid input may already overlap the same pitch; refuse a change that re-pairs it.
    if shape(&notes(&edited)) != shape(&expected) {
        return Err("Overlaps another note of the same pitch".into());
    }
    *source = edited;
    Ok(())
}

fn measure(music: &Music, track: usize, number: u64) -> Result<Measure, String> {
    let time = MusicalTime::new(music);
    Measure::new(
        time.map(track).ok_or("Missing measure map")?,
        music.division,
        number,
    )
}
fn bounds(measure: &Measure, step: usize) -> (u64, u64) {
    let start = measure.steps[step];
    (
        start,
        measure.steps.get(step + 1).copied().unwrap_or(measure.end),
    )
}
fn containing(map: &MeterMap, division: u16, tick: u64) -> Result<Measure, String> {
    let number = u64::try_from(map.position(tick).measure).map_err(|_| "Measure overflow")?;
    Measure::new(map, division, number)
}
fn on_grid(map: &MeterMap, division: u16, tick: u64) -> bool {
    containing(map, division, tick).is_ok_and(|m| m.steps.contains(&tick))
}
/// The first step boundary after `tick`, continuing into the next measure.
fn next(map: &MeterMap, division: u16, tick: u64) -> Result<u64, String> {
    let m = containing(map, division, tick)?;
    Ok(m.steps.iter().copied().find(|&s| s > tick).unwrap_or(m.end))
}
/// The last step boundary before `tick`, continuing into the previous measure.
fn previous(map: &MeterMap, division: u16, tick: u64) -> Result<Option<u64>, String> {
    let m = containing(map, division, tick)?;
    if let Some(s) = m.steps.iter().copied().rev().find(|&s| s < tick) {
        return Ok(Some(s));
    }
    if m.number <= 1 {
        return Ok(None);
    }
    Ok(Measure::new(map, division, m.number - 1)?
        .steps
        .last()
        .copied())
}
fn event(tick: u64, status: u8, pitch: u8, velocity: u8) -> Event {
    Event {
        tick,
        delta: 0,
        status,
        meta_type: 0,
        data: vec![pitch, velocity],
    }
}
/// Note On goes after everything at its tick; Note Off goes before a Note On of the same
/// channel and key at its tick, so a note ending where the next begins does not cut it.
/// End of Track stays last.
fn place(track: &mut Track, event: Event) {
    let end = track
        .events
        .pop_if(|e| e.status == 0xff && e.meta_type == 0x2f);
    let key = (event.status & 15, event.data[0]);
    let at = if note_on(&event) {
        track.events.partition_point(|e| e.tick <= event.tick)
    } else {
        track
            .events
            .iter()
            .position(|e| {
                e.tick > event.tick
                    || (e.tick == event.tick && note_on(e) && (e.status & 15, e.data[0]) == key)
            })
            .unwrap_or(track.events.len())
    };
    track.events.insert(at, event);
    if let Some(mut end) = end {
        end.tick = end.tick.max(track.events.last().map_or(0, |e| e.tick));
        track.events.push(end);
    }
}

#[cfg(test)]
mod tests;
