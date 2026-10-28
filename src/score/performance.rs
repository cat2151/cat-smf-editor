//! Flattened Song performance; native parameter windows retain exact discontinuities.
use super::{
    pattern::{self, render::ParameterInterval},
    scene,
};
use crate::{
    edit,
    model::{Event, Music, Track},
};
use std::collections::BTreeMap;
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0performance-parameter\0";
pub const OWNER: &[u8] = b"\x7dcat-smf-editor\0performance-owner\0";
pub fn is_owner(t: &Track) -> bool {
    t.events
        .iter()
        .any(|e| e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(OWNER))
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u8,
    end: u64,
    baseline: f64,
    curve: crate::edit::parameter_curve::Curve,
}
pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
fn event(w: &ParameterInterval) -> Result<Event, String> {
    Ok(Event {
        tick: w.start,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: [
            PREFIX,
            serde_json::to_vec(&Saved {
                version: 1,
                end: w.end,
                baseline: w.baseline,
                curve: w.curve.clone(),
            })
            .map_err(|e| e.to_string())?
            .as_slice(),
        ]
        .concat(),
    })
}
pub fn read(m: &Music) -> Result<BTreeMap<usize, Vec<ParameterInterval>>, String> {
    let mut result = BTreeMap::new();
    for (track, t) in m.tracks.iter().enumerate() {
        let mut windows: Vec<ParameterInterval> = vec![];
        let headers = t
            .events
            .iter()
            .filter(|e| e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(OWNER))
            .collect::<Vec<_>>();
        if headers.len() > 1
            || (track == 0 && !headers.is_empty())
            || headers
                .first()
                .is_some_and(|e| e.tick != 0 || e.data.strip_prefix(OWNER) != Some(&[1][..]))
        {
            return Err("Invalid performance owner".into());
        }
        for e in t.events.iter().filter(|e| owned(e)) {
            let saved: Saved = serde_json::from_slice(&e.data[PREFIX.len()..])
                .map_err(|e| format!("Invalid performance parameter: {e}"))?;
            saved.curve.validate()?;
            if saved.version != 1
                || headers.is_empty()
                || saved.curve.points.is_empty()
                || saved.end <= e.tick
                || saved.end > t.events.last().map_or(0, |e| e.tick)
                || !saved.baseline.is_finite()
                || !(saved.curve.minimum..=saved.curve.maximum).contains(&saved.baseline)
                || saved
                    .curve
                    .points
                    .iter()
                    .any(|p| p.tick > saved.end - e.tick)
            {
                return Err("Invalid performance parameter window".into());
            }
            if windows.iter().any(|w| {
                let a = &w.curve.target;
                let b = &saved.curve.target;
                a.layer == b.layer
                    && a.plugin_format == b.plugin_format
                    && a.plugin_id == b.plugin_id
                    && a.parameter_id == b.parameter_id
                    && w.start < saved.end
                    && e.tick < w.end
            }) {
                return Err("Overlapping performance parameter windows".into());
            }
            windows.push(ParameterInterval {
                start: e.tick,
                end: saved.end,
                baseline: saved.baseline,
                curve: saved.curve,
            });
        }
        if !windows.is_empty() || !headers.is_empty() {
            windows.sort_by_key(|w| w.start);
            result.insert(track, windows);
        }
    }
    Ok(result)
}
fn arrangement_meta(e: &Event) -> bool {
    pattern::owned(e)
        || pattern::automation::owned(e)
        || scene::owned(e)
        || (e.status == 0xff
            && e.meta_type == 0x7f
            && e.data.starts_with(pattern::automation::BASELINES))
}
fn end(track: &mut Track, tick: u64) -> Result<(), String> {
    track
        .events
        .retain(|e| !(e.status == 0xff && e.meta_type == 0x2f));
    track.events.push(Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    });
    track.events.sort_by_key(|e| e.tick);
    edit::refresh_deltas(track)
}
pub fn song(m: &Music) -> Result<Music, String> {
    let catalog = pattern::Catalog::read(m)?;
    let collection = scene::read(m)?;
    if collection.song_length == 0 {
        return Err("Song has no explicit performance length".into());
    }
    let (occurrences, length) = scene::arrangement::occurrences(m, None)?;
    let expanded = pattern::render::expand(m, &catalog, &occurrences, length)?;
    let mut conductor = m.tracks.first().ok_or("Missing conductor")?.clone();
    conductor.events.retain(|e| {
        e.status == 0xff
            && !arrangement_meta(e)
            && !owned(e)
            && !e.data.starts_with(crate::app_meta::AUTO_SAVE)
    });
    conductor.events.retain(|e| e.tick <= length);
    end(&mut conductor, length)?;
    let mut tracks = vec![conductor];
    for (&owner, events) in &expanded.events {
        let source = &m.tracks[owner];
        if source
            .events
            .iter()
            .any(|e| e.data.starts_with(super::macro_curve::PREFIX))
        {
            return Err(
                "Container macro variation curves are not pattern expressions; content retained"
                    .into(),
            );
        }
        let mut track = Track {
            events: source
                .events
                .iter()
                .filter(|e| {
                    e.status == 0xff
                        && !matches!(e.meta_type, 0x2f | 0x51 | 0x58 | 1 | 5 | 6 | 7)
                        && !arrangement_meta(e)
                        && !owned(e)
                })
                .cloned()
                .collect(),
        };
        track.events.push(Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data: [OWNER, &[1]].concat(),
        });
        track.events.extend(events.iter().cloned());
        for w in expanded.parameters.get(&owner).into_iter().flatten() {
            track.events.push(event(w)?);
        }
        end(&mut track, length)?;
        tracks.push(track);
    }
    let result = Music {
        format: 1,
        division: m.division,
        tracks,
    };
    read(&result)?;
    Ok(result)
}
pub fn describe(data: &[u8]) -> String {
    serde_json::from_slice::<Saved>(&data[PREFIX.len()..])
        .map(|s| {
            format!(
                "{} / {} · end {} · baseline {}",
                s.curve.path, s.curve.name, s.end, s.baseline
            )
        })
        .unwrap_or_else(|_| "invalid".into())
}
#[cfg(test)]
mod tests;
