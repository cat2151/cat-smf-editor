use super::{OwnerPlan, Sequence};
use std::sync::Arc;

pub struct RuntimePlan {
    pub owner: Arc<OwnerPlan>,
    pub origin: f64,
    pub rate: u32,
    origin_frame: u64,
    boundaries: Vec<u64>,
    events: Vec<SourceEvent>,
}
pub struct SourceEvent {
    pub sample: u64,
    pub tick: f64,
    pub midi: Option<u32>,
}
fn frame(seconds: f64, rate: u32) -> u64 {
    (seconds * f64::from(rate)).round().max(0.0) as u64
}
impl RuntimePlan {
    pub fn new(owner: Arc<OwnerPlan>, sequence: &Sequence, origin: f64, rate: u32) -> Self {
        let origin_frame = frame(origin, rate);
        let normalized = |tick: f64| {
            if (tick - tick.round()).abs() < 1e-7 {
                tick.round()
            } else {
                tick
            }
        };
        let origin_tick = owner.tempo.fractional_tick(origin);
        let mut events: Vec<_> = owner
            .boundaries
            .iter()
            .filter(|(tick, _)| *tick as f64 >= origin_tick)
            .map(|(tick, seconds)| SourceEvent {
                sample: frame(*seconds, rate),
                tick: *tick as f64,
                midi: None,
            })
            .collect();
        let mut boundaries: Vec<_> = owner
            .boundaries
            .iter()
            .map(|(_, seconds)| frame(*seconds, rate))
            .collect();
        let timeline = crate::play::event_timeline::Timeline::new(sequence, origin, rate);
        for event in &timeline.events {
            let sample = origin_frame.saturating_add(event.sample);
            boundaries.push(sample);
            events.push(SourceEvent {
                sample,
                tick: normalized(owner.tempo.fractional_tick(event.seconds)),
                midi: Some(event.word),
            });
        }
        if sequence.end.is_finite() {
            boundaries.push(frame(origin + sequence.end, rate));
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        events.sort_by(|a, b| {
            a.sample
                .cmp(&b.sample)
                .then_with(|| a.tick.total_cmp(&b.tick))
        });
        Self {
            owner,
            origin,
            rate,
            origin_frame,
            boundaries,
            events,
        }
    }
    fn absolute(&self, elapsed: u64) -> u64 {
        self.origin_frame.saturating_add(elapsed)
    }
    pub fn next(&self, elapsed: u64, max: usize) -> usize {
        let sample = self.absolute(elapsed);
        let grid = 64 - (sample % 64) as usize;
        let i = self.boundaries.partition_point(|&b| b <= sample);
        self.boundaries.get(i).map_or(max.min(grid), |next| {
            max.min(grid)
                .min((next - sample).min(usize::MAX as u64) as usize)
        })
    }
    pub fn applies(&self, elapsed: u64) -> bool {
        let sample = self.absolute(elapsed);
        elapsed == 0 || sample.is_multiple_of(64) || self.boundaries.binary_search(&sample).is_ok()
    }
    pub fn tick(&self, elapsed: u64) -> f64 {
        let events = self.events_at(elapsed);
        if !self.is_grid(elapsed)
            && let Some(first) = events.first()
            && events.iter().all(|event| event.tick == first.tick)
        {
            return first.tick;
        }
        self.sample_tick(elapsed)
    }
    pub fn sample_tick(&self, elapsed: u64) -> f64 {
        self.owner.tempo.fractional_tick(if elapsed == 0 {
            self.origin
        } else {
            self.absolute(elapsed) as f64 / f64::from(self.rate)
        })
    }
    pub fn is_grid(&self, elapsed: u64) -> bool {
        elapsed == 0 || self.absolute(elapsed).is_multiple_of(64)
    }
    pub fn events_at(&self, elapsed: u64) -> &[SourceEvent] {
        let sample = self.absolute(elapsed);
        let start = self.events.partition_point(|e| e.sample < sample);
        let end = self.events.partition_point(|e| e.sample <= sample);
        &self.events[start..end]
    }
}
