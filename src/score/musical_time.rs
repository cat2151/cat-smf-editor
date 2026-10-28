//! Exact PPQN arithmetic, independent of presentation and parsing.
use crate::model::Music;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub measure: u128,
    pub beat: u128,
    pub tick_numerator: u128,
    pub tick_denominator: u128,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.tick_denominator == 1 {
            write!(f, "{}:{}:{}", self.measure, self.beat, self.tick_numerator)
        } else {
            write!(
                f,
                "{}:{}:{}/{}",
                self.measure, self.beat, self.tick_numerator, self.tick_denominator
            )
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Meter {
    numerator: u8,
    exponent: u8,
}
impl Meter {
    fn denominator(self) -> u128 {
        1u128 << self.exponent
    }
}
struct Segment {
    tick: u64,
    measure: u128,
    meter: Meter,
}
pub struct MeterMap {
    ppqn: u16,
    segments: Vec<Segment>,
    pub warnings: Vec<String>,
}
/// A compact run of exact beat boundaries, including fractional MIDI ticks.
pub struct BeatRun {
    pub tick: u64,
    pub step: u128,
    pub denominator: u128,
    pub count: u128,
    measure: u128,
    numerator: u8,
}
impl BeatRun {
    pub fn beats_per_measure(&self) -> u128 {
        u128::from(self.numerator)
    }
    pub fn position(&self, index: u128) -> Position {
        Position {
            measure: self.measure + index / u128::from(self.numerator),
            beat: index % u128::from(self.numerator) + 1,
            tick_numerator: 0,
            tick_denominator: 1,
        }
    }
}
impl MeterMap {
    pub fn signature(&self, tick: u64) -> (u8, u8) {
        let segment = &self.segments[self.segments.partition_point(|s| s.tick <= tick) - 1];
        (segment.meter.numerator, segment.meter.exponent)
    }
    /// Absolute tick of an MBT position (fractional boundaries round up).
    pub fn tick_at(&self, position: &Position) -> Option<u64> {
        let segment = self
            .segments
            .iter()
            .rev()
            .find(|s| s.measure <= position.measure)?;
        let beats = (position.measure - segment.measure)
            .checked_mul(u128::from(segment.meter.numerator))?
            .checked_add(position.beat.checked_sub(1)?)?;
        let denominator = segment.meter.denominator();
        if position.tick_denominator == 0 || denominator % position.tick_denominator != 0 {
            return None;
        }
        let offset = beats
            .checked_mul(u128::from(self.ppqn) * 4)?
            .checked_add(
                position
                    .tick_numerator
                    .checked_mul(denominator / position.tick_denominator)?,
            )?
            .div_ceil(denominator);
        u64::try_from(u128::from(segment.tick).checked_add(offset)?).ok()
    }

    pub fn adjacent_beat(
        &self,
        position: &Position,
        forward: bool,
        until: u64,
    ) -> Option<Position> {
        let current = (position.measure, position.beat);
        let mut previous = None;
        for run in self.beat_runs(until) {
            let start = run.position(0);
            let index = if current < (start.measure, start.beat) {
                0
            } else {
                (position.measure - start.measure) * u128::from(run.numerator) + position.beat - 1
            };
            if forward {
                let next = if current < (start.measure, start.beat) {
                    0
                } else {
                    index + 1
                };
                if next < run.count {
                    return Some(run.position(next));
                }
            } else {
                let before = index.min(run.count);
                if before > 0 {
                    previous = Some(run.position(before - 1));
                }
                if index < run.count {
                    break;
                }
            }
        }
        previous
    }

    pub fn beat_runs(&self, until: u64) -> Vec<BeatRun> {
        self.segments
            .iter()
            .enumerate()
            .take_while(|(_, s)| s.tick <= until)
            .map(|(i, s)| {
                let denominator = s.meter.denominator();
                let step = u128::from(self.ppqn) * 4;
                let count = if let Some(next) = self.segments.get(i + 1).filter(|n| n.tick <= until)
                {
                    (u128::from(next.tick - s.tick) * denominator).div_ceil(step)
                } else {
                    u128::from(until - s.tick) * denominator / step + 1
                };
                BeatRun {
                    tick: s.tick,
                    step,
                    denominator,
                    count,
                    measure: s.measure,
                    numerator: s.meter.numerator,
                }
            })
            .collect()
    }
    pub fn position(&self, tick: u64) -> Position {
        let index = self.segments.partition_point(|s| s.tick <= tick) - 1;
        let segment = &self.segments[index];
        // tick (64 bits) * denominator (at most 2^63) fits u128.
        let units = u128::from(tick - segment.tick) * segment.meter.denominator();
        let beat_units = u128::from(self.ppqn) * 4;
        let beat_count = units / beat_units;
        let remainder = units % beat_units;
        let denominator = segment.meter.denominator();
        let divisor = gcd(remainder, denominator);
        Position {
            measure: segment.measure + beat_count / u128::from(segment.meter.numerator),
            beat: beat_count % u128::from(segment.meter.numerator) + 1,
            tick_numerator: remainder / divisor,
            tick_denominator: denominator / divisor,
        }
    }

    fn build(music: &Music, track: Option<usize>) -> Self {
        let mut changes = Vec::new();
        let mut warnings = Vec::new();
        for (t, source) in music.tracks.iter().enumerate() {
            if track.is_some_and(|selected| selected != t) {
                continue;
            }
            for (i, event) in source.events.iter().enumerate() {
                if event.status != 0xff || event.meta_type != 0x58 {
                    continue;
                }
                if !valid_time_signature(&event.data) {
                    warnings.push(format!(
                        "Invalid time signature: track {}, event {} (tick {})",
                        t + 1,
                        i + 1,
                        event.tick
                    ));
                    continue;
                }
                changes.push((
                    event.tick,
                    t,
                    i,
                    Meter {
                        numerator: event.data[0],
                        exponent: event.data[1],
                    },
                ));
            }
        }
        changes.sort_by_key(|c| (c.0, c.1, c.2));
        let mut segments = vec![Segment {
            tick: 0,
            measure: 1,
            meter: Meter {
                numerator: 4,
                exponent: 2,
            },
        }];
        let mut index = 0;
        while index < changes.len() {
            let start = index;
            let tick = changes[index].0;
            while index < changes.len() && changes[index].0 == tick {
                index += 1;
            }
            let meter = changes[index - 1].3;
            if changes[start..index].iter().any(|c| c.3 != meter) {
                warnings.push(format!(
                    "Conflicting time signatures at tick {tick}; last track/event wins"
                ));
            }
            let previous = segments.last().unwrap();
            if tick == 0 {
                segments[0].meter = meter;
            } else if previous.meter != meter {
                let distance = u128::from(tick - previous.tick) * previous.meter.denominator();
                let bar = u128::from(music.division) * 4 * u128::from(previous.meter.numerator);
                let measure = previous.measure + distance.div_ceil(bar);
                segments.push(Segment {
                    tick,
                    measure,
                    meter,
                });
            }
        }
        Self {
            ppqn: music.division,
            segments,
            warnings,
        }
    }
}

/// Data of a Time Signature meta event that `MeterMap` reads instead of warning about.
pub fn valid_time_signature(data: &[u8]) -> bool {
    data.len() == 4 && data[0] != 0 && data[1] <= 63
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

pub struct MusicalTime {
    maps: Vec<MeterMap>,
    independent: bool,
}
impl MusicalTime {
    pub fn new(music: &Music) -> Self {
        let independent = music.format == 2;
        let maps = if music.division == 0 || music.division & 0x8000 != 0 {
            Vec::new()
        } else if independent {
            (0..music.tracks.len())
                .map(|t| MeterMap::build(music, Some(t)))
                .collect()
        } else {
            vec![MeterMap::build(music, None)]
        };
        Self { maps, independent }
    }
    pub fn map(&self, track: usize) -> Option<&MeterMap> {
        self.maps.get(if self.independent { track } else { 0 })
    }
}
