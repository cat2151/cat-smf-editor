//! Apply row timing in musical ticks, including tempo changes.
use super::Sequence;
use cat_smf_editor::score::{
    audition_timing::TempoMap,
    drums::{Settings, NOTES},
};

impl Sequence {
    pub fn drums(self, settings: Option<&Settings>, division: u16, tempo: &TempoMap) -> Self {
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
        self.swing(division, tempo, |pitch| {
            let row = &settings.rows[NOTES.iter().position(|&p| p == pitch)?];
            row.swing_enabled.then_some(row.swing)
        })
    }

    pub fn kit_swing(
        self,
        kit: Option<&cat_smf_editor::drum_kit::Kit>,
        division: u16,
        tempo: &TempoMap,
    ) -> Self {
        let Some(kit) = kit.filter(|k| k.swing.values().any(|s| s.enabled && s.amount != 50))
        else {
            return self;
        };
        self.swing(division, tempo, |pitch| {
            let swing = kit.swing.get(&pitch)?;
            swing.enabled.then_some(swing.amount)
        })
    }

    fn swing(mut self, division: u16, tempo: &TempoMap, amount: impl Fn(u8) -> Option<u8>) -> Self {
        let time = |seconds: f64, pitch: u8| {
            let Some(swing) = amount(pitch).filter(|&s| s != 50) else {
                return seconds;
            };
            if !seconds.is_finite() {
                return seconds;
            }
            let row = cat_smf_editor::score::drums::Row {
                layer: 1,
                output: pitch,
                velocity: 100,
                swing_enabled: true,
                swing,
            };
            let tick = row.swung_tick(tempo.fractional_tick(seconds), division);
            let whole = tick.floor() as u64;
            tempo.seconds_at(whole) + tempo.duration(whole, 1) * tick.fract()
        };
        for note in &mut self.notes {
            let start = time(self.origin + note.start, note.pitch);
            let end = time(self.origin + note.start + note.duration, note.pitch);
            note.start = (start - self.origin).max(0.0);
            note.duration = (end - start).max(0.0);
        }
        for event in &mut self.events {
            let status = (event.word >> 20) & 15;
            if !matches!(status, 8 | 9) {
                continue;
            }
            let pitch = ((event.word >> 8) & 127) as u8;
            event.time = time(event.time, pitch).max(self.origin);
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
