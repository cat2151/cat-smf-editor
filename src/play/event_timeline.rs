//! Immutable, sample-addressed note events prepared outside the audio callback.
use super::audio_sequence::Sequence;

pub struct Timeline {
    pub origin: f64,
    pub events: Vec<Event>,
    pub end: u64,
}

#[cfg(test)]
mod tests;
pub struct Event {
    pub sample: u64,
    pub seconds: f64,
    pub word: u32,
}
pub fn frame(seconds: f64, rate: u32) -> u64 {
    (seconds * f64::from(rate)).round().max(0.0) as u64
}
impl Timeline {
    pub fn new(sequence: &Sequence, origin: f64, rate: u32) -> Self {
        let base = frame(origin, rate);
        let offset = origin - sequence.origin;
        Self {
            origin,
            events: sequence
                .events
                .iter()
                .map(|event| {
                    let seconds = event.time + offset;
                    Event {
                        sample: frame(seconds, rate).saturating_sub(base),
                        seconds,
                        word: event.word,
                    }
                })
                .collect(),
            end: frame(origin + sequence.end, rate).saturating_sub(base),
        }
    }
    pub fn events_at(&self, sample: u64) -> &[Event] {
        let start = self.events.partition_point(|e| e.sample < sample);
        let end = self.events.partition_point(|e| e.sample <= sample);
        &self.events[start..end]
    }
    pub fn next(&self, sample: u64, max: usize) -> usize {
        let index = self.events.partition_point(|e| e.sample <= sample);
        let next = self
            .events
            .get(index)
            .map_or(self.end, |e| e.sample)
            .min(self.end);
        if next > sample {
            max.min((next - sample).min(usize::MAX as u64) as usize)
        } else {
            max
        }
    }
}
