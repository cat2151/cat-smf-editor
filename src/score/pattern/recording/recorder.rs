//! Pads pressed during a session, split into one take per loop lap.
use super::{control, note, operations, pitches, ticks, write, Pad, Range, Settings};
use crate::message;
use crate::model::Music;
use std::time::Duration;

pub struct Recorder {
    division: u16,
    settings: Settings,
    /// Free: Rec. Loop: the end of the count-in.
    origin: Option<Duration>,
    /// How many held pads sound each pitch.
    sounding: [u8; 128],
    /// Notes each pad started, so releasing stops them even after the settings change.
    held: Vec<(Pad, Vec<u8>)>,
    /// Free keeps one lap; Loop keeps one per lap number, empty ones included.
    laps: Vec<Lap>,
    /// Presses that sounded, which step the velocity sweep.
    presses: usize,
    /// The velocity of the last sounding press.
    velocity: u8,
    /// The velocity each sounding pitch started with.
    velocities: [u8; 128],
}

#[derive(Clone)]
struct Lap {
    events: Vec<crate::model::Event>,
    on: [Option<u64>; 128],
    released: [u64; 128],
}

impl Lap {
    fn new() -> Self {
        Self {
            events: vec![],
            on: [None; 128],
            released: [0; 128],
        }
    }
}

impl Recorder {
    pub fn new(division: u16, settings: Settings) -> Result<Self, String> {
        if division == 0 || division & 0x8000 != 0 {
            return Err(message::RECORDING_REQUIRES_PPQN_TIMING.into());
        }
        if settings.tempo == 0 {
            return Err(message::RECORDING_REQUIRES_A_POSITIVE_TEMPO.into());
        }
        if matches!(settings.range, Range::Loop { measures: 0 }) {
            return Err("Loop録音には1小節以上が必要です".into());
        }
        Ok(Self {
            division,
            settings,
            origin: None,
            sounding: [0; 128],
            held: vec![],
            laps: vec![],
            presses: 0,
            velocity: 0,
            velocities: [0; 128],
        })
    }
    /// The velocity of the last press that sounded; 0 before the first.
    pub fn velocity(&self) -> u8 {
        self.velocity
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
    /// Pitches that start sounding, all at [`Self::velocity`].
    pub fn press(&mut self, pad: Pad, at: Duration) -> Result<Vec<u8>, String> {
        if self.held(pad) {
            return Ok(vec![]);
        }
        let notes = pitches(&self.settings, pad);
        if notes.is_empty() {
            return Ok(vec![]);
        }
        self.velocity = self.settings.velocity.of_press(self.presses);
        self.presses += 1;
        let mut started = vec![];
        for &pitch in &notes {
            self.sounding[usize::from(pitch)] += 1;
            if self.sounding[usize::from(pitch)] == 1 {
                started.push(pitch);
                self.velocities[usize::from(pitch)] = self.velocity;
                self.record(pitch, self.velocity, at)?;
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
                self.record(pitch, 0, at)?;
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
    /// A Note On of `velocity`, or a Note Off for 0.
    fn record(&mut self, pitch: u8, velocity: u8, at: Duration) -> Result<(), String> {
        let on = velocity > 0;
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
            lap.events.push(note(tick, pitch, velocity));
        } else if let Some(start) = lap.on[p].take() {
            let tick = local.max(
                start
                    .checked_add(1)
                    .ok_or(message::RECORDING_TIME_OVERFLOW)?,
            );
            lap.released[p] = tick;
            lap.events.push(note(tick, pitch, 0));
        }
        Ok(())
    }
    /// Notes sounding across a lap boundary end with the lap and restart at the next lap's 0.
    fn advance(&mut self, lap: u64) {
        let length = self.length().unwrap_or(0);
        let velocities = self.velocities;
        while self.laps.len() as u64 <= lap {
            let mut next = Lap::new();
            if let Some(last) = self.laps.last_mut() {
                for pitch in 0..128u8 {
                    let p = usize::from(pitch);
                    if last.on[p].take().is_some() {
                        last.released[p] = length;
                        last.events.push(note(length, pitch, 0));
                        next.on[p] = Some(0);
                        next.events.push(note(0, pitch, velocities[p]));
                    }
                }
            }
            self.laps.push(next);
        }
    }
    /// One pattern per lap that holds notes, created in lap order, with the CC1 of a sweep
    /// started at `sweep`; the caller supplies a clone.
    pub fn commit(
        &self,
        music: &mut Music,
        source: Option<usize>,
        sweep: Duration,
    ) -> Result<Vec<u64>, String> {
        if !self.held.is_empty() || self.laps.iter().any(|l| l.on.iter().any(Option::is_some)) {
            return Err("録音中に押さえたままのnoteがあります".into());
        }
        if music.division != self.division {
            return Err("録音の分割が変わりました".into());
        }
        let laps = self
            .laps
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.events.is_empty());
        if laps.clone().next().is_none() {
            return Err(message::RECORDING_IS_EMPTY.into());
        }
        let source = match source {
            Some(track) => track,
            None => operations::create_source(music)?,
        };
        let measure = u64::from(self.division) * 4;
        let grid = if self.settings.quantize {
            Some(crate::edit::quantize::sixteenth(self.division)?)
        } else {
            None
        };
        laps.map(|(i, lap)| {
            let notes = match grid {
                Some(grid) => super::quantize::events(lap.events.clone(), grid, self.length()),
                None => lap.events.clone(),
            };
            let end = notes.iter().map(|e| e.tick).max().unwrap_or(0);
            let count = match self.settings.range {
                Range::Free => end.div_ceil(measure).max(1),
                Range::Loop { measures } => u64::from(measures),
            };
            let start = self.length().map_or(0, |l| i as u64 * l);
            // CC1 first, so a value on a note's tick reaches the note.
            let mut events = self.controls(sweep, start, start + count * measure)?;
            events.extend(notes);
            write(music, source, count, events)
        })
        .collect()
    }
    /// CC1 from tick `start` to `end` after the origin, at ticks from `start`.
    fn controls(
        &self,
        sweep: Duration,
        start: u64,
        end: u64,
    ) -> Result<Vec<crate::model::Event>, String> {
        let Some(origin) = self.origin else {
            return Ok(vec![]);
        };
        let time = |tick: u64| {
            origin
                + Duration::from_nanos(
                    (u128::from(tick) * u128::from(self.settings.tempo) * 1000
                        / u128::from(self.division))
                    .min(u128::from(u64::MAX)) as u64,
                )
        };
        let mut events = vec![];
        for (at, value) in self
            .settings
            .modulation
            .changes(sweep, time(start), time(end))
        {
            let tick = ticks(self.division, self.settings.tempo, origin, at)?.unwrap_or(0);
            if (start..end).contains(&tick) {
                control(&mut events, tick - start, value);
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests;
