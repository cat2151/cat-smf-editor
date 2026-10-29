//! A transient 120 BPM take, committed as ordinary pattern note events.
use super::{operations, Catalog, Meter};
use crate::{
    edit,
    model::{Event, Music},
};
use std::time::Duration;

#[derive(Clone)]
pub struct Take {
    division: u16,
    origin: Option<Duration>,
    held: [Option<u64>; 128],
    released: [u64; 128],
    events: Vec<Event>,
    end: u64,
}

impl Take {
    pub fn new(division: u16) -> Result<Self, String> {
        if division == 0 || division & 0x8000 != 0 {
            return Err("Recording requires PPQN timing".into());
        }
        Ok(Self {
            division,
            origin: None,
            held: [None; 128],
            released: [0; 128],
            events: vec![],
            end: 0,
        })
    }
    pub fn started(&self) -> bool {
        self.origin.is_some()
    }
    pub fn held(&self, pitch: u8) -> bool {
        self.held
            .get(usize::from(pitch))
            .is_some_and(Option::is_some)
    }
    fn tick(&self, at: Duration) -> Result<u64, String> {
        let nanos = at.saturating_sub(self.origin.unwrap_or(at)).as_nanos();
        u64::try_from((nanos * u128::from(self.division) * 2 + 500_000_000) / 1_000_000_000)
            .map_err(|_| "Recording time overflow".into())
    }
    pub fn press(&mut self, pitch: u8, at: Duration) -> Result<bool, String> {
        if pitch >= 128 || self.held(pitch) {
            return Ok(false);
        }
        self.origin.get_or_insert(at);
        let tick = self.tick(at)?.max(self.released[usize::from(pitch)]);
        self.held[usize::from(pitch)] = Some(tick);
        self.events.push(note(tick, true, pitch));
        Ok(true)
    }
    pub fn release(&mut self, pitch: u8, at: Duration) -> Result<bool, String> {
        if !self.held(pitch) {
            return Ok(false);
        }
        let start = self.held[usize::from(pitch)].unwrap();
        let tick = self
            .tick(at)?
            .max(start.checked_add(1).ok_or("Recording time overflow")?);
        self.held[usize::from(pitch)] = None;
        self.released[usize::from(pitch)] = tick;
        self.end = self.end.max(tick);
        self.events.push(note(tick, false, pitch));
        Ok(true)
    }
    pub fn release_all(&mut self, at: Duration) -> Result<Vec<u8>, String> {
        let mut released = vec![];
        for pitch in 0..128 {
            if self.release(pitch, at)? {
                released.push(pitch);
            }
        }
        Ok(released)
    }
    /// The caller supplies a clone so failed validation never alters the document.
    pub fn commit(&self, music: &mut Music, source: Option<usize>) -> Result<u64, String> {
        if !self.started() || self.held.iter().any(Option::is_some) {
            return Err("Recording is empty or has held notes".into());
        }
        if music.division != self.division {
            return Err("Recording division changed".into());
        }
        let measure = u64::from(self.division) * 4;
        let count = self.end.div_ceil(measure).max(1);
        if count > 100_000 {
            return Err("Recording exceeds the pattern measure limit".into());
        }
        let source = match source {
            Some(track) => track,
            None => operations::create_source(music)?,
        };
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
        let mut events = self.events.clone();
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
