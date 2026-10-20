//! Convert note lifetimes to seconds through the source track's tempo map.
use crate::model::Music;

pub struct TempoMap {
    segments: Vec<(u64, f64, f64)>,
}
impl TempoMap {
    pub fn new(music: &Music, track: usize) -> Self {
        if music.division & 0x8000 != 0 {
            let fps = -i16::from((music.division >> 8) as i8);
            let fps = if fps == 29 {
                30_000.0 / 1001.0
            } else {
                f64::from(fps)
            };
            return Self {
                segments: vec![(0, 0.0, 1.0 / (fps * f64::from(music.division & 255)))],
            };
        }
        if music.division == 0 {
            return Self {
                segments: vec![(0, 0.0, 0.0)],
            };
        }
        let mut changes = Vec::new();
        for (t, source) in music.tracks.iter().enumerate() {
            if music.format == 2 && t != track {
                continue;
            }
            for (i, event) in source.events.iter().enumerate() {
                if event.status == 0xff && event.meta_type == 0x51 && event.data.len() == 3 {
                    let tempo = (u32::from(event.data[0]) << 16)
                        | (u32::from(event.data[1]) << 8)
                        | u32::from(event.data[2]);
                    if tempo != 0 {
                        changes.push((event.tick, t, i, tempo));
                    }
                }
            }
        }
        changes.sort_by_key(|c| (c.0, c.1, c.2));
        let mut segments = vec![(0, 0.0, 0.5 / f64::from(music.division))];
        for (tick, _, _, tempo) in changes {
            let &(previous, seconds, slope) = segments.last().unwrap();
            let seconds = seconds + (tick - previous) as f64 * slope;
            let slope = f64::from(tempo) / 1_000_000.0 / f64::from(music.division);
            if tick == previous {
                *segments.last_mut().unwrap() = (tick, seconds, slope);
            } else {
                segments.push((tick, seconds, slope));
            }
        }
        Self { segments }
    }
    fn at(&self, tick: u64) -> f64 {
        let i = self.segments.partition_point(|s| s.0 <= tick) - 1;
        let (start, seconds, slope) = self.segments[i];
        seconds + (tick - start) as f64 * slope
    }
    /// Convert elapsed audio time back to a source tick, including tempo changes.
    pub fn tick_after(&self, start: u64, elapsed: f64) -> u64 {
        let seconds = self.at(start) + elapsed.max(0.0);
        let i = self
            .segments
            .partition_point(|s| s.1 <= seconds)
            .saturating_sub(1);
        let (tick, time, slope) = self.segments[i];
        if slope <= 0.0 {
            return start;
        }
        tick.saturating_add(((seconds - time) / slope).floor().max(0.0) as u64)
    }
    /// Quarter-note clock; SMPTE and zero division have no defined beat here.
    pub fn beat_clock(&self, seconds: f64, division: u16) -> Option<(f64, f64)> {
        if division == 0 || division & 0x8000 != 0 || !seconds.is_finite() {
            return None;
        }
        let i = self
            .segments
            .partition_point(|s| s.1 <= seconds)
            .saturating_sub(1);
        let (tick, start, slope) = self.segments[i];
        if slope <= 0.0 {
            return None;
        }
        let ticks = f64::from(division);
        Some((
            (tick as f64 + (seconds - start).max(0.0) / slope) / ticks,
            1.0 / (slope * ticks),
        ))
    }
    pub fn duration(&self, start: u64, duration: u64) -> f64 {
        self.at(start.saturating_add(duration)) - self.at(start)
    }
}

pub fn duration_seconds(music: &Music, track: usize, start: u64, duration: u64) -> f64 {
    TempoMap::new(music, track).duration(start, duration)
}
