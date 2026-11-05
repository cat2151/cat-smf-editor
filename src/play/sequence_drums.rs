//! Apply row timing in musical ticks, including tempo changes.
use super::Sequence;
use cat_smf_editor::score::{
    audition_timing::TempoMap,
    drums::{Settings, NOTES},
};

impl Sequence {
    pub fn drums(mut self, settings: Option<&Settings>, division: u16, tempo: &TempoMap) -> Self {
        let Some(settings) = settings else {
            return self;
        };
        if settings
            .rows
            .iter()
            .all(|r| !r.swing_enabled || r.swing == 50)
        {
            return self;
        }
        let time = |seconds: f64, row: usize| {
            if !settings.rows[row].swing_enabled
                || settings.rows[row].swing == 50
                || !seconds.is_finite()
            {
                return seconds;
            }
            let tick = settings.rows[row].swung_tick(tempo.fractional_tick(seconds), division);
            let whole = tick.floor() as u64;
            tempo.seconds_at(whole) + tempo.duration(whole, 1) * tick.fract()
        };
        for note in &mut self.notes {
            if let Some(row) = NOTES.iter().position(|&pitch| pitch == note.pitch) {
                let start = time(self.origin + note.start, row);
                let end = time(self.origin + note.start + note.duration, row);
                note.start = (start - self.origin).max(0.0);
                note.duration = (end - start).max(0.0);
            }
        }
        for event in &mut self.events {
            let status = (event.word >> 20) & 15;
            if !matches!(status, 8 | 9) {
                continue;
            }
            let pitch = ((event.word >> 8) & 127) as u8;
            if let Some(row) = NOTES.iter().position(|&pitch2| pitch2 == pitch) {
                event.time = time(event.time, row).max(self.origin);
            }
        }
        self.notes.sort_by(|a, b| a.start.total_cmp(&b.start));
        self.events.sort_by(|a, b| a.time.total_cmp(&b.time));
        self.truncate(self.end);
        self
    }
}

#[cfg(test)]
#[path = "sequence_drum_tests.rs"]
mod tests;
