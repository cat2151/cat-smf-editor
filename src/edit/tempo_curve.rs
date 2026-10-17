//! A tempo curve: quadratic Bézier segments through tempo points, stored as a Sequencer
//! Specific meta in the conductor track and rendered into tempo events at every Note On
//! and Note Off of any track.
use crate::{
    edit::{insert, refresh_deltas},
    model::{Event, Music},
    score::note_view::{note_off, note_on},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0tempo-curve\0";
const MAX_TEMPO: u32 = 0xff_ffff;
/// Bisection steps solving `x(s) = tick`; enough for f64 precision.
const STEPS: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub tick: u64,
    pub bpm: f64,
}

/// `controls[i]` bends the segment from `points[i]` to `points[i + 1]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub version: u8,
    pub points: Vec<Point>,
    pub controls: Vec<Point>,
}

/// BPM range a tempo event (1..=0xFFFFFF µs per quarter note) can express.
pub fn bpm_range() -> std::ops::RangeInclusive<f64> {
    60_000_000.0 / f64::from(MAX_TEMPO)..=60_000_000.0
}

pub(super) fn microseconds(bpm: f64) -> u32 {
    (60_000_000.0 / bpm)
        .round()
        .clamp(1.0, f64::from(MAX_TEMPO)) as u32
}

impl Curve {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported tempo curve schema".into());
        }
        if self.points.is_empty() {
            return Err("A tempo curve needs at least one tempo point".into());
        }
        if self.controls.len() != self.points.len() - 1 {
            return Err("A tempo curve needs one control point per segment".into());
        }
        if self.points.windows(2).any(|w| w[0].tick >= w[1].tick) {
            return Err("Tempo points must be in increasing tick order".into());
        }
        let mut segments = self.points.windows(2).zip(&self.controls);
        if segments.any(|(w, c)| !(w[0].tick..=w[1].tick).contains(&c.tick)) {
            return Err("A control point must stay inside its segment".into());
        }
        if self
            .points
            .iter()
            .chain(&self.controls)
            .any(|p| !bpm_range().contains(&p.bpm))
        {
            return Err("A tempo curve BPM is out of the SMF range".into());
        }
        Ok(())
    }

    /// BPM at `tick`; constant before the first and after the last tempo point.
    pub fn bpm_at(&self, tick: f64) -> f64 {
        let (first, last) = (self.points[0], self.points[self.points.len() - 1]);
        if tick <= first.tick as f64 {
            return first.bpm;
        }
        if tick >= last.tick as f64 {
            return last.bpm;
        }
        let i = self.points.partition_point(|p| p.tick as f64 <= tick) - 1;
        let (a, b, c) = (self.points[i], self.points[i + 1], self.controls[i]);
        let bezier = |s: f64, a: f64, c: f64, b: f64| {
            (1.0 - s) * (1.0 - s) * a + 2.0 * s * (1.0 - s) * c + s * s * b
        };
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..STEPS {
            let middle = (low + high) / 2.0;
            if bezier(middle, a.tick as f64, c.tick as f64, b.tick as f64) < tick {
                low = middle;
            } else {
                high = middle;
            }
        }
        bezier((low + high) / 2.0, a.bpm, c.bpm, b.bpm)
    }
}

pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}

fn is_tempo(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x51
}

pub fn read(music: &Music) -> Result<Option<Curve>, String> {
    let mut found = None;
    for (track, t) in music.tracks.iter().enumerate() {
        for e in t.events.iter().filter(|e| owned(e)) {
            if music.format != 1 || track != 0 || e.tick != 0 || found.is_some() {
                return Err(
                    "A tempo curve requires one tick-zero metadata event in format 1 track 0"
                        .into(),
                );
            }
            let curve: Curve =
                serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
            curve.validate()?;
            found = Some(curve);
        }
    }
    Ok(found)
}

/// Stores `curve` as the conductor's metadata, or removes it for `None`; tempo events stay.
pub fn write(music: &mut Music, curve: Option<&Curve>) -> Result<(), String> {
    if music.format != 1 || music.tracks.is_empty() {
        return Err("A tempo curve requires SMF format 1".into());
    }
    read(music)?;
    let data = match curve {
        Some(curve) => {
            curve.validate()?;
            let mut data = PREFIX.to_vec();
            data.extend(serde_json::to_vec(curve).map_err(|e| e.to_string())?);
            Some(data)
        }
        None => None,
    };
    let conductor = &mut music.tracks[0];
    conductor.events.retain(|e| !owned(e));
    if let Some(data) = data {
        conductor.events.insert(
            0,
            Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x7f,
                data,
            },
        );
    }
    refresh_deltas(conductor)
}

/// (tick, µs per quarter note) of the curve's tempo at tick 0 and at each Note On / Note Off
/// tick of any track, skipping values equal to the previous one.
pub fn tempos(music: &Music, curve: &Curve) -> Vec<(u64, u32)> {
    let ticks: BTreeSet<u64> = std::iter::once(0)
        .chain(
            music
                .tracks
                .iter()
                .flat_map(|t| &t.events)
                .filter(|e| note_on(e) || note_off(e))
                .map(|e| e.tick),
        )
        .collect();
    let mut previous = None;
    let mut out = Vec::new();
    for tick in ticks {
        let tempo = microseconds(curve.bpm_at(tick as f64));
        if previous != Some(tempo) {
            previous = Some(tempo);
            out.push((tick, tempo));
        }
    }
    out
}

/// Replaces every conductor tempo event with [`tempos`].
pub fn regenerate(music: &mut Music, curve: &Curve) -> Result<(), String> {
    if music.format != 1 || music.tracks.is_empty() {
        return Err("A tempo curve requires SMF format 1".into());
    }
    curve.validate()?;
    let tempos = tempos(music, curve);
    let conductor = &mut music.tracks[0];
    conductor.events.retain(|e| !is_tempo(e));
    for (tick, tempo) in tempos {
        insert(
            conductor,
            Event {
                tick,
                delta: 0,
                status: 0xff,
                meta_type: 0x51,
                data: tempo.to_be_bytes()[1..].to_vec(),
            },
        );
    }
    refresh_deltas(conductor)
}

#[cfg(test)]
mod tests;
