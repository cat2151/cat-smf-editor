use cat_smf_editor::{
    model::Event,
    score::musical_time::{MeterMap, Position},
};

/// Measured once per track, including events outside the visible viewport.
#[derive(Default)]
pub struct TimeColumns {
    pub delta: usize,
    pub absolute: usize,
    /// Characters of the longest raw preview.
    pub raw: usize,
    pub pitch_range: Option<(u8, u8)>,
    measure: usize,
    beat: usize,
    tick: usize,
}

impl TimeColumns {
    pub fn new(events: &[Event], map: Option<&MeterMap>) -> Self {
        let mut widths = Self {
            delta: 1,
            absolute: 1,
            raw: 1,
            pitch_range: None,
            measure: 1,
            beat: 1,
            tick: 1,
        };
        for event in events {
            if cat_smf_editor::score::note_view::note_on(event) {
                let pitch = event.data[0];
                widths.pitch_range =
                    Some(widths.pitch_range.map_or((pitch, pitch), |(min, max)| {
                        (min.min(pitch), max.max(pitch))
                    }));
            }
            widths.delta = widths.delta.max(event.delta.to_string().len());
            widths.absolute = widths.absolute.max(event.tick.to_string().len());
            widths.raw = widths
                .raw
                .max(super::row::raw_preview(event).chars().count());
            if let Some(map) = map {
                let p = map.position(event.tick);
                widths.measure = widths.measure.max(p.measure.to_string().len());
                widths.beat = widths.beat.max(p.beat.to_string().len());
                widths.tick = widths.tick.max(tick_text(&p).len());
            }
        }
        widths
    }

    pub fn position(&self, p: &Position) -> String {
        format!(
            "{:>measure$}:{:>beat$}:{:>tick$}",
            p.measure,
            p.beat,
            tick_text(p),
            measure = self.measure,
            beat = self.beat,
            tick = self.tick
        )
    }

    /// A row without events omits the tick, and a whole empty measure also omits the beat.
    pub fn rest_position(&self, p: &Position, whole_measure: bool) -> String {
        if whole_measure {
            format!("{:>measure$}", p.measure, measure = self.measure)
        } else {
            format!(
                "{:>measure$}:{:>beat$}",
                p.measure,
                p.beat,
                measure = self.measure,
                beat = self.beat
            )
        }
    }

    pub fn position_chars(&self) -> usize {
        self.measure + self.beat + self.tick + 2
    }
}

fn tick_text(p: &Position) -> String {
    if p.tick_denominator == 1 {
        p.tick_numerator.to_string()
    } else {
        format!("{}/{}", p.tick_numerator, p.tick_denominator)
    }
}

#[cfg(test)]
mod tests;
