use super::*;
use crate::score::same_measure;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Catalog {
    pub tracks: BTreeMap<u64, usize>,
    pub sources: BTreeSet<usize>,
    pub patterns: BTreeMap<u64, Definition>,
    pub identities: BTreeSet<u64>,
}
impl Catalog {
    pub fn read(music: &Music) -> Result<Self, String> {
        let mut c = Self::default();
        for (track, t) in music.tracks.iter().enumerate() {
            let mut has_id = false;
            for e in &t.events {
                if let Some(Record::Track { id, .. }) = record(e)? {
                    if has_id || e.tick != 0 || !c.identities.insert(id) {
                        return Err("Duplicate or misplaced track identity".into());
                    }
                    c.tracks.insert(id, track);
                    has_id = true;
                    c.sources.insert(track);
                }
            }
        }
        for (track, t) in music.tracks.iter().enumerate() {
            for e in &t.events {
                if let Some(Record::Definition {
                    id, name, meters, ..
                }) = record(e)?
                {
                    if !c.sources.contains(&track)
                        || name.trim().is_empty()
                        || !c.identities.insert(id)
                    {
                        return Err("Invalid pattern identity or container".into());
                    }
                    body::meter_music(&meters, music.division)?;
                    let container = *c.tracks.iter().find(|(_, t)| **t == track).unwrap().0;
                    let p = Definition {
                        id,
                        name,
                        container,
                        track,
                        tick: e.tick,
                        meters,
                    };
                    let range = p.range()?;
                    let curves = super::automation::curves(music, &p)?;
                    let baselines = super::automation::baselines(music, track)?;
                    for curve in &curves {
                        if !baselines.iter().any(|b|matches!(b,super::automation::Baseline::Parameter {target,value_space,minimum,maximum,..} if target==&curve.target && value_space==&curve.value_space && *minimum==curve.minimum && *maximum==curve.maximum)) {return Err("Missing or incompatible container parameter baseline".into());}
                    }
                    body::extract(&t.events, range)?;
                    if t.events
                        .iter()
                        .any(|e| same_measure::owned(e) && (range.0..=range.1).contains(&e.tick))
                    {
                        return Err("Pattern contains same measure reference".into());
                    }
                    if c.patterns.values().any(|other| {
                        other.track == track
                            && other
                                .range()
                                .is_ok_and(|r| range.0 <= r.1 && range.1 >= r.0)
                    }) {
                        return Err("Pattern storage ranges overlap or have no boundary gap".into());
                    }
                    c.patterns.insert(id, p);
                }
            }
        }
        let scenes = super::super::scene::read(music)?;
        for id in scenes
            .scenes
            .iter()
            .map(|s| s.id)
            .chain(
                scenes
                    .scenes
                    .iter()
                    .flat_map(|s| s.cells.iter().map(|p| p.id)),
            )
            .chain(scenes.song.iter().map(|p| p.id))
            .chain(scenes.lanes.iter().map(|l| l.id))
        {
            if !c.identities.insert(id) {
                return Err("Duplicate identity across document roles".into());
            }
        }
        super::super::scene::validate(music, &c)?;
        Ok(c)
    }
    pub fn fresh_id(&self) -> u64 {
        loop {
            let id = fastrand::u64(1..);
            if !self.identities.contains(&id) {
                return id;
            }
        }
    }
}
