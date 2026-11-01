//! Container-owned pattern bodies. Storage ticks have no musical meaning.
use crate::message;
use crate::model::{Event, Music};
use serde::{Deserialize, Serialize};
pub mod automation;
pub mod body;
mod catalog;
pub mod editing;
pub mod operations;
pub mod recording;
pub mod render;
pub use super::scene::Meter;
pub use catalog::Catalog;
/// Marks the track as a sound container; its tick is 0.
pub const CONTAINER_PREFIX: &[u8] = b"\x7dcat-smf-editor\0pattern-container\0";
/// Defines one pattern; its tick is where the stored body starts.
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0pattern\0";
pub const VERSION: u8 = 3;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerRecord {
    pub version: u8,
    pub id: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Normal,
    /// Collects what was played on the virtual keyboard.
    Capture,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternRecord {
    pub version: u8,
    pub id: u64,
    pub kind: Kind,
    pub name: String,
    pub meters: Vec<Meter>,
}
#[derive(Clone, Debug)]
pub enum Record {
    Container(ContainerRecord),
    Pattern(PatternRecord),
}
fn meta(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f
}
pub fn owned(e: &Event) -> bool {
    meta(e) && (e.data.starts_with(PREFIX) || e.data.starts_with(CONTAINER_PREFIX))
}
pub fn record(e: &Event) -> Result<Option<Record>, String> {
    if !meta(e) {
        return Ok(None);
    }
    let invalid = |e: serde_json::Error| format!("pattern metaが不正です: {e}");
    let (r, version, id) = if let Some(json) = e.data.strip_prefix(CONTAINER_PREFIX) {
        let r: ContainerRecord = serde_json::from_slice(json).map_err(invalid)?;
        let (version, id) = (r.version, r.id);
        (Record::Container(r), version, id)
    } else if let Some(json) = e.data.strip_prefix(PREFIX) {
        let r: PatternRecord = serde_json::from_slice(json).map_err(invalid)?;
        let (version, id) = (r.version, r.id);
        (Record::Pattern(r), version, id)
    } else {
        return Ok(None);
    };
    if version != VERSION || id == 0 {
        return Err("patternのversionか識別子が不正です".into());
    }
    Ok(Some(r))
}
pub fn event(tick: u64, r: &Record) -> Event {
    let (prefix, json) = match r {
        Record::Container(r) => (CONTAINER_PREFIX, serde_json::to_vec(r)),
        Record::Pattern(r) => (PREFIX, serde_json::to_vec(r)),
    };
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: [prefix, json.unwrap().as_slice()].concat(),
    }
}
pub fn is_source(music: &Music, track: usize) -> bool {
    music.tracks.get(track).is_some_and(|t| {
        t.events
            .iter()
            .any(|e| matches!(record(e), Ok(Some(Record::Container(_)))))
    })
}
#[derive(Clone, Debug)]
pub struct Definition {
    pub id: u64,
    pub kind: Kind,
    pub name: String,
    pub container: u64,
    pub track: usize,
    pub tick: u64,
    pub meters: Vec<Meter>,
}
impl Definition {
    pub fn length(&self) -> Result<u64, String> {
        body::length(&self.meters)
    }
    pub fn range(&self) -> Result<(u64, u64), String> {
        Ok((
            self.tick,
            self.tick
                .checked_add(self.length()?)
                .ok_or(message::PATTERN_RANGE_OVERFLOW)?,
        ))
    }
    pub fn meter_music(&self, division: u16) -> Result<Music, String> {
        body::meter_music(&self.meters, division)
    }
}
pub fn describe(data: &[u8]) -> String {
    match record(&Event {
        tick: 0,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: data.to_vec(),
    }) {
        Ok(Some(Record::Container(r))) => format!("{}", r.id),
        Ok(Some(Record::Pattern(r))) => format!(
            "{} · {} · {} · {} measures",
            r.name,
            r.kind.label(),
            r.id,
            r.meters.len()
        ),
        _ => crate::message::INVALID.into(),
    }
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Normal => "Normal",
            Kind::Capture => "Capture",
        }
    }
}
#[cfg(test)]
mod editing_tests;
#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tests;
