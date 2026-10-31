//! Pads pressed during a session, split into one take per loop lap.
use super::{note, operations, pitches, ticks, write, Pad, Range, Settings, Start};
use crate::model::Music;
use std::time::Duration;

pub struct Recorder {
    division: u16,
    settings: Settings,
    /// Free: Rec, or the first sounding press with `Start::Key`. Loop: the end of the count-in.
    origin: Option<Duration>,
    /// How many held pads sound each pitch.
    sounding: [u8; 128],
    /// Notes each pad started, so releasing stops them even after the settings change.
    held: Vec<(Pad, Vec<u8>)>,
    /// Free keeps one lap; Loop keeps one per lap number, empty ones included.
    laps: Vec<Lap>,
}

#[derive(Clone)]
struct Lap {
    events: Vec<crate::model::Event>,
    on: [Option<u64>; 128],
    released: [u64; 128],
    end: u64,
}

impl Lap {
    fn new() -> Self {
        Self {
            events: vec![],
            on: [None; 128],
            released: [0; 128],
            end: 0,
        }
    }
}

impl Recorder {
    pub fn new(division: u16, settings: Settings) -> Result<Self, String> {
        if division == 0 || division & 0x8000 != 0 {
            return Err("Recording requires PPQN timing".into());
        }
        if settings.tempo == 0 {
            return Err("Recording requires a positive tempo".into());
        }
        if matches!(settings.range, Range::Loop { measures: 0 }) {
            return Err("Loop recording requires at least one measure".into());
        }
        Ok(Self {
            division,
            settings,
            origin: None,
            sounding: [0; 128],
            held: vec![],
            laps: vec![],
        })
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    /// Tempo and range are read at every event, so only pads, mode and bases change once started.
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    pub fn started(&self) -> bool {
        self.origin.is_some()
    }
    /// Loop recording counts laps from `origin`; presses before it sound but are not recorded.
    pub fn start(&mut self, origin: Duration) {
        self.origin.get_or_insert(origin);
    }
    pub fn origin(&self) -> Option<Duration> {
        self.origin
    }
    /// Some lap holds a note event, so committing creates at least one pattern.
    pub fn recorded(&self) -> bool {
        self.laps.iter().any(|l| !l.events.is_empty())
    }
    pub fn held(&self, pad: Pad) -> bool {
        self.held.iter().any(|(p, _)| *p == pad)
    }
    pub fn held_pads(&self) -> impl Iterator<Item = Pad> + '_ {
        self.held.iter().map(|(p, _)| *p)
    }
    pub fn sounding(&self, pitch: u8) -> bool {
        self.sounding
            .get(usize::from(pitch))
            .is_some_and(|&n| n > 0)
    }
    /// Pitches that start sounding.
    pub fn press(&mut self, pad: Pad, at: Duration) -> Result<Vec<u8>, String> {
        if self.held(pad) {
            return Ok(vec![]);
        }
        let notes = pitches(&self.settings, pad);
        if notes.is_empty() {
            return Ok(vec![]);
        }
        if self.settings.range == Range::Free && self.settings.start == Start::Key {
            self.origin.get_or_insert(at);
        }
        let mut started = vec![];
        for &pitch in &notes {
            self.sounding[usize::from(pitch)] += 1;
            if self.sounding[usize::from(pitch)] == 1 {
                started.push(pitch);
                self.record(pitch, true, at)?;
            }
        }
        self.held.push((pad, notes));
        Ok(started)
    }
    /// Pitches that stop sounding.
    pub fn release(&mut self, pad: Pad, at: Duration) -> Result<Vec<u8>, String> {
        let Some(index) = self.held.iter().position(|(p, _)| *p == pad) else {
            return Ok(vec![]);
        };
        let (_, notes) = self.held.remove(index);
        let mut stopped = vec![];
        for pitch in notes {
            self.sounding[usize::from(pitch)] -= 1;
            if self.sounding[usize::from(pitch)] == 0 {
                stopped.push(pitch);
                self.record(pitch, false, at)?;
            }
        }
        Ok(stopped)
    }
    pub fn release_all(&mut self, at: Duration) -> Result<Vec<u8>, String> {
        let mut stopped = vec![];
        while let Some(&(pad, _)) = self.held.first() {
            stopped.extend(self.release(pad, at)?);
        }
        Ok(stopped)
    }
    fn length(&self) -> Option<u64> {
        match self.settings.range {
            Range::Free => None,
            Range::Loop { measures } => Some(u64::from(measures) * 4 * u64::from(self.division)),
        }
    }
    fn record(&mut self, pitch: u8, on: bool, at: Duration) -> Result<(), String> {
        let Some(origin) = self.origin else {
            return Ok(());
        };
        let Some(t) = ticks(self.division, self.settings.tempo, origin, at)? else {
            return Ok(());
        };
        let length = self.length();
        let current = self.laps.len().saturating_sub(1) as u64;
        // A release exactly on a lap boundary ends the earlier lap instead of opening the next.
        let lap = length.map_or(0, |l| {
            let lap = if on { t / l } else { t.saturating_sub(1) / l };
            lap.max(current)
        });
        self.advance(lap);
        let local = t.saturating_sub(length.map_or(0, |l| lap * l));
        let p = usize::from(pitch);
        let lap = self.laps.last_mut().unwrap();
        if on {
            let tick = local.max(lap.released[p]);
            // Never start a note on the lap's closing tick: a pattern cannot hold its Note On there.
            if length.is_some_and(|l| tick >= l) {
                return Ok(());
            }
            lap.on[p] = Some(tick);
            lap.events.push(note(tick, true, pitch));
        } else if let Some(start) = lap.on[p].take() {
            let tick = local.max(start.checked_add(1).ok_or("Recording time overflow")?);
            lap.released[p] = tick;
            lap.end = lap.end.max(tick);
            lap.events.push(note(tick, false, pitch));
        }
        Ok(())
    }
    /// Notes sounding across a lap boundary end with the lap and restart at the next lap's 0.
    fn advance(&mut self, lap: u64) {
        let length = self.length().unwrap_or(0);
        while self.laps.len() as u64 <= lap {
            let mut next = Lap::new();
            if let Some(last) = self.laps.last_mut() {
                for pitch in 0..128u8 {
                    let p = usize::from(pitch);
                    if last.on[p].take().is_some() {
                        last.released[p] = length;
                        last.end = length;
                        last.events.push(note(length, false, pitch));
                        next.on[p] = Some(0);
                        next.events.push(note(0, true, pitch));
                    }
                }
            }
            self.laps.push(next);
        }
    }
    /// One pattern per lap that holds notes, created in lap order; the caller supplies a clone.
    pub fn commit(&self, music: &mut Music, source: Option<usize>) -> Result<Vec<u64>, String> {
        if !self.held.is_empty() || self.laps.iter().any(|l| l.on.iter().any(Option::is_some)) {
            return Err("Recording has held notes".into());
        }
        if music.division != self.division {
            return Err("Recording division changed".into());
        }
        let laps = self.laps.iter().filter(|l| !l.events.is_empty());
        if laps.clone().next().is_none() {
            return Err("Recording is empty".into());
        }
        let source = match source {
            Some(track) => track,
            None => operations::create_source(music)?,
        };
        let measure = u64::from(self.division) * 4;
        laps.map(|lap| {
            let count = match self.settings.range {
                Range::Free => lap.end.div_ceil(measure).max(1),
                Range::Loop { measures } => u64::from(measures),
            };
            write(music, source, count, lap.events.clone())
        })
        .collect()
    }
}

#[cfg(test)]
mod tests;
