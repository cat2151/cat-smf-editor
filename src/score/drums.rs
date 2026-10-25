//! Four GM lanes; the score keeps GM pitches and the renderer maps each lane.
use crate::{
    edit,
    model::{Event, Track},
    score::note_view::note_on,
};
use serde::{Deserialize, Serialize};

pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0drums\0";
pub const NOTES: [u8; 4] = [36, 38, 42, 39];
pub const NAMES: [&str; 4] = ["Kick", "Snare", "Hi-Hat", "Percussion"];

#[cfg(test)]
mod measure_tests;
mod measures;
pub use measures::{rebuild, toggle_measure, Measure};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub layer: u64,
    pub output: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u8,
    pub rows: [Row; 4],
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            rows: std::array::from_fn(|i| Row {
                layer: i as u64 + 1,
                output: 60,
            }),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.rows.iter().any(|r| r.layer == 0 || r.output > 127)
            || self
                .rows
                .iter()
                .enumerate()
                .any(|(i, r)| self.rows[..i].iter().any(|p| p.layer == r.layer))
        {
            return Err("Invalid drum sequencer settings".into());
        }
        Ok(())
    }
    pub fn pitch(&self, layer: u64, input: u8) -> Option<u8> {
        self.rows
            .iter()
            .enumerate()
            .find(|(i, r)| r.layer == layer && NOTES[*i] == input)
            .map(|(_, r)| r.output)
    }
    pub fn validate_layers(
        &self,
        layers: &[crate::instrument_layers::Layer],
    ) -> Result<(), String> {
        self.validate()?;
        if layers.len() != 4
            || self
                .rows
                .iter()
                .any(|r| !layers.iter().any(|l| l.id == r.layer))
        {
            return Err("Drum sequencer requires its four instrument rows".into());
        }
        Ok(())
    }
}
pub(crate) fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
pub fn read(track: &Track) -> Result<Option<Settings>, String> {
    let mut found = None;
    for e in track.events.iter().filter(|e| owned(e)) {
        if found.is_some() || e.tick != 0 || e.data.len() > 4096 {
            return Err("Duplicate, misplaced or oversized drum metadata".into());
        }
        let settings: Settings =
            serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
        settings.validate()?;
        found = Some(settings);
    }
    Ok(found)
}
pub fn write(track: &mut Track, settings: Option<&Settings>) -> Result<(), String> {
    read(track)?;
    if let Some(s) = settings {
        s.validate()?;
    }
    let mut next = track.clone();
    next.events.retain(|e| !owned(e));
    if let Some(s) = settings {
        let mut data = PREFIX.to_vec();
        data.extend(serde_json::to_vec(s).map_err(|e| e.to_string())?);
        next.events.insert(
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
    edit::refresh_deltas(&mut next)?;
    *track = next;
    Ok(())
}

/// Rational boundaries avoid accumulating rounding error for PPQN not divisible by four.
pub fn tick(division: u16, step: u64) -> Result<u64, String> {
    if division < 4 || division & 0x8000 != 0 {
        return Err("Drum editing requires PPQN >= 4".into());
    }
    step.checked_mul(u64::from(division))
        .map(|v| v / 4)
        .ok_or("Drum step exceeds tick range".into())
}
pub fn active(track: &Track, division: u16, row: usize, step: u64) -> bool {
    let (Ok(start), Ok(end)) = (tick(division, step), tick(division, step.saturating_add(1)))
    else {
        return false;
    };
    track
        .events
        .iter()
        .any(|e| note_on(e) && e.data[0] == NOTES[row] && e.tick >= start && e.tick < end)
}
pub fn toggle(track: &mut Track, division: u16, row: usize, step: u64) -> Result<(), String> {
    let pitch = *NOTES.get(row).ok_or("Invalid drum row")?;
    let start = tick(division, step)?;
    let end = tick(division, step.checked_add(1).ok_or("Drum step overflow")?)?;
    let remove: Vec<_> = track
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            (note_on(e) && e.data[0] == pitch && e.tick >= start && e.tick < end).then_some(i)
        })
        .collect();
    if !remove.is_empty() {
        edit::event_range::delete(track, &remove, true)?;
    } else {
        edit::insert(
            track,
            Event {
                tick: start,
                delta: 0,
                status: 0x99,
                meta_type: 0,
                data: vec![pitch, 100],
            },
        );
        let eot = track
            .events
            .pop_if(|e| e.status == 0xff && e.meta_type == 0x2f);
        // A newly inserted off must precede an existing next-step on of the same pitch.
        let at = track.events.partition_point(|e| e.tick < end);
        track.events.insert(
            at,
            Event {
                tick: end,
                delta: 0,
                status: 0x89,
                meta_type: 0,
                data: vec![pitch, 0],
            },
        );
        if let Some(mut eot) = eot {
            eot.tick = eot.tick.max(end);
            track.events.push(eot);
        }
    }
    edit::refresh_deltas(track)
}

#[cfg(test)]
mod tests;
