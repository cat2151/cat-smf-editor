//! Container-owned pattern bodies. Storage ticks have no musical meaning.
use crate::model::{Event, Music};
use serde::{Deserialize, Serialize};
pub mod automation;
pub mod body;
mod catalog;
pub mod editing;
pub mod operations;
pub mod render;
pub use super::scene::Meter;
pub use catalog::Catalog;
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0pattern\0";
pub const VERSION: u8 = 2;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Record {
    Track {
        version: u8,
        id: u64,
    },
    Definition {
        version: u8,
        id: u64,
        name: String,
        meters: Vec<Meter>,
    },
}
pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
pub fn record(e: &Event) -> Result<Option<Record>, String> {
    if !owned(e) {
        return Ok(None);
    }
    let r: Record = serde_json::from_slice(&e.data[PREFIX.len()..])
        .map_err(|e| format!("Invalid pattern meta: {e}"))?;
    let (version, id) = match &r {
        Record::Track { version, id } | Record::Definition { version, id, .. } => (*version, *id),
    };
    if version != VERSION || id == 0 {
        return Err("Invalid pattern version or identity".into());
    }
    Ok(Some(r))
}
pub fn event(tick: u64, r: &Record) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: [PREFIX, serde_json::to_vec(r).unwrap().as_slice()].concat(),
    }
}
pub fn is_source(music: &Music, track: usize) -> bool {
    music.tracks.get(track).is_some_and(|t| {
        t.events
            .iter()
            .any(|e| matches!(record(e), Ok(Some(Record::Track { .. }))))
    })
}
#[derive(Clone, Debug)]
pub struct Definition {
    pub id: u64,
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
                .ok_or("Pattern range overflow")?,
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
        Ok(Some(Record::Track { id, .. })) => format!("Container {id}"),
        Ok(Some(Record::Definition {
            id, name, meters, ..
        })) => format!("{name} · {id} · {} measures", meters.len()),
        _ => "invalid".into(),
    }
}
#[cfg(test)]
mod editing_tests;
#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tests;
