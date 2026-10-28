//! Named relative placements, with intrinsic measure lengths and stable lane identities.
use crate::{
    model::{Event, Music, Track},
    score::{musical_time::MusicalTime, pattern::Catalog, same_measure},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub mod arrangement;
pub mod persistence;
pub mod projection;
pub mod sizing;
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0scenes\0";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meter {
    pub numerator: u8,
    pub exponent: u8,
    pub ticks: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub id: u64,
    pub lane: u64,
    /// Zero-based measure inside the Scene.
    pub measure: u64,
    pub pattern: u64,
    /// Commented out: saved and occupying its cell, but not performed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: u64,
    pub name: String,
    pub meters: Vec<Meter>,
    pub lanes: Vec<u64>,
    pub cells: Vec<Cell>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub version: u8,
    pub lanes: Vec<arrangement::Lane>,
    pub scenes: Vec<Scene>,
    pub song_lanes: Vec<u64>,
    pub song: Vec<arrangement::Placement>,
    pub song_length: u64,
    #[serde(skip)]
    pub pattern_lengths: std::collections::BTreeMap<u64, u64>,
}
impl Default for Collection {
    fn default() -> Self {
        Self {
            version: persistence::VERSION,
            scenes: vec![],
            lanes: vec![],
            song: vec![],
            song_lanes: vec![],
            song_length: 0,
            pattern_lengths: Default::default(),
        }
    }
}
pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
pub use persistence::{read, write};
impl Scene {
    pub fn range(&self, measure: u128) -> Result<(u64, u64), String> {
        let index = usize::try_from(measure.checked_sub(1).ok_or("Invalid scene measure")?)
            .map_err(|_| "Invalid scene measure")?;
        let meter = self
            .meters
            .get(index)
            .ok_or("Scene measure is outside its length")?;
        let start = self.meters[..index]
            .iter()
            .try_fold(0u64, |sum, m| sum.checked_add(m.ticks))
            .ok_or("Scene length overflow")?;
        Ok((
            start,
            start
                .checked_add(meter.ticks)
                .ok_or("Scene length overflow")?,
        ))
    }
    pub fn meter_music(&self, division: u16) -> Result<Music, String> {
        let mut events = vec![];
        let mut tick = 0u64;
        for meter in &self.meters {
            if meter.numerator == 0 || meter.exponent > 63 || meter.ticks == 0 {
                return Err("Invalid scene meter".into());
            }
            events.push(Event {
                tick,
                delta: 0,
                status: 0xff,
                meta_type: 0x58,
                data: vec![meter.numerator, meter.exponent, 24, 8],
            });
            tick = tick
                .checked_add(meter.ticks)
                .ok_or("Scene length overflow")?;
        }
        Ok(Music {
            format: 1,
            division,
            tracks: vec![Track { events }],
        })
    }
}
pub fn validate(music: &Music, catalog: &Catalog) -> Result<(), String> {
    let collection = read(music)?;
    let mut ids = BTreeSet::new();
    for scene in &collection.scenes {
        if scene.id == 0
            || !ids.insert(scene.id)
            || scene.name.trim().is_empty()
            || scene.meters.is_empty()
            || scene.lanes.is_empty()
        {
            return Err("Invalid scene identity or length".into());
        }
        let mut lanes = BTreeSet::new();
        for id in &scene.lanes {
            if !lanes.insert(*id) || !collection.lanes.iter().any(|l| l.id == *id) {
                return Err("Missing or invalid scene lane".into());
            }
        }
        let grid = scene.meter_music(music.division)?;
        let grid_time = MusicalTime::new(&grid);
        let map = grid_time.map(0).ok_or("Scene requires PPQN timing")?;
        for i in 0..scene.meters.len() {
            if same_measure::bounds(map, i as u128 + 1)? != scene.range(i as u128 + 1)? {
                return Err("Scene meter cannot represent its exact measure boundaries".into());
            }
        }
        let mut cells = Vec::new();
        for cell in &scene.cells {
            if !lanes.contains(&cell.lane)
                || cells.iter().any(|&(lane, a, b)| {
                    lane == cell.lane
                        && cell.measure < b
                        && cell.measure.saturating_add(
                            catalog
                                .patterns
                                .get(&cell.pattern)
                                .map_or(0, |p| p.meters.len() as u64),
                        ) > a
                })
            {
                return Err("Invalid or duplicate scene cell".into());
            }
            let destination = scene.range(u128::from(cell.measure) + 1)?;
            let p = catalog
                .patterns
                .get(&cell.pattern)
                .ok_or("Missing scene pattern")?;
            cells.push((
                cell.lane,
                cell.measure,
                cell.measure
                    .checked_add(p.meters.len() as u64)
                    .ok_or("Scene placement overflow")?,
            ));
            let source = p.range()?;
            if scene
                .meters
                .get(cell.measure as usize..cell.measure as usize + p.meters.len())
                != Some(p.meters.as_slice())
                || destination
                    .0
                    .checked_add(source.1 - source.0)
                    .is_none_or(|end| end > scene.range(scene.meters.len() as u128).unwrap().1)
            {
                return Err("Scene pattern measure lengths differ".into());
            }
        }
    }
    arrangement::validate(music, catalog, &collection)?;
    for scene in collection.scenes.iter().map(|s| Some(s.id)).chain(
        std::iter::once(None).filter(|_| collection.song_length > 0 || !collection.song.is_empty()),
    ) {
        let (occurrences, end) = arrangement::occurrences_in(music, catalog, &collection, scene)?;
        crate::score::pattern::render::expand(music, catalog, &occurrences, end)?;
    }
    Ok(())
}
pub fn describe(data: &[u8]) -> String {
    serde_json::from_slice::<Collection>(&data[PREFIX.len()..])
        .map(|c| {
            let commented = |n: usize| {
                if n == 0 {
                    String::new()
                } else {
                    format!(", {n} commented out")
                }
            };
            format!(
                "{} · Song {} lanes, {} ticks{}",
                c.scenes
                    .iter()
                    .map(|s| format!(
                        "{} [{} measures, {} lanes{}]",
                        s.name,
                        s.meters.len(),
                        s.lanes.len(),
                        commented(s.cells.iter().filter(|p| p.disabled).count())
                    ))
                    .collect::<Vec<_>>()
                    .join(" · "),
                c.song_lanes.len(),
                c.song_length,
                commented(c.song.iter().filter(|p| p.disabled).count())
            )
        })
        .unwrap_or_else(|_| "invalid".into())
}
#[cfg(test)]
mod tests;
