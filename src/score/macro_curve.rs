//! Variation references on an instrument owner; expanded into ordinary macro lanes.
use crate::{
    edit::{
        parameter_curve::{Curve, Point},
        refresh_deltas,
    },
    model::{Event, Track},
};
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0macro-variation-curve\0";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u8,
    layer: u64,
    plugin_format: String,
    plugin_id: String,
    points: Vec<Point>,
    controls: Vec<Point>,
}
fn encode(curves: &[Curve]) -> Result<Vec<u8>, String> {
    let saved = curves
        .iter()
        .map(|c| {
            let (format, id) = c
                .target
                .plugin_id
                .split_once(':')
                .ok_or("Invalid variation instrument identity")?;
            Ok(Saved {
                version: c.version,
                layer: c.target.layer,
                plugin_format: format.into(),
                plugin_id: id.into(),
                points: c.points.clone(),
                controls: c.controls.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_vec(&saved).map_err(|e| e.to_string())
}
fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
pub fn read(track: &Track) -> Result<Vec<Curve>, String> {
    let mut events = track.events.iter().filter(|e| owned(e));
    let Some(e) = events.next() else {
        return Ok(vec![]);
    };
    if e.tick != 0 || e.data.len() > 1024 * 1024 || events.next().is_some() {
        return Err("Misplaced or duplicate macro variation curves".into());
    }
    let saved: Vec<Saved> =
        serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
    let values = super::macro_values::read(track)?;
    let curves: Vec<_> = saved
        .into_iter()
        .map(|s| Curve {
            version: s.version,
            target: crate::edit::parameter_curve::Target {
                group: 1,
                layer: s.layer,
                plugin_format: "CLAP".into(),
                plugin_id: format!("{}:{}", s.plugin_format, s.plugin_id),
                parameter_id: "macro-variation".into(),
            },
            value_space: crate::edit::parameter_curve::ValueSpace::Plain,
            minimum: 0.0,
            maximum: values
                .variations
                .iter()
                .find(|v| v.layer == s.layer)
                .map_or(127.0, |v| v.snapshots.len().saturating_sub(1).max(1) as f64),
            name: "Macro variation curve".into(),
            path: "Macros".into(),
            points: s.points,
            controls: s.controls,
        })
        .collect();
    let mut layers = std::collections::HashSet::new();
    for c in &curves {
        c.validate()?;
        if c.minimum != 0.0
            || c.maximum > 127.0
            || c.maximum < 1.0
            || c.maximum.fract() != 0.0
            || !layers.insert(c.target.layer)
            || c.target.parameter_id != "macro-variation"
            || c.points.iter().any(|p| p.value.fract() != 0.0)
        {
            return Err("Invalid macro variation curve".into());
        }
    }
    Ok(curves)
}
pub fn describe(json: &[u8]) -> String {
    match serde_json::from_slice::<Vec<Saved>>(json) {
        Ok(curves) => curves
            .iter()
            .map(|c| {
                format!(
                    "Layer {} / {} {} · points {} · controls {}",
                    c.layer,
                    c.plugin_format,
                    c.plugin_id,
                    c.points
                        .iter()
                        .map(|p| format!("{}: Variation {}", p.tick, p.value + 1.0))
                        .collect::<Vec<_>>()
                        .join(" → "),
                    c.controls
                        .iter()
                        .map(|p| format!("{}: {}", p.tick, p.value + 1.0))
                        .collect::<Vec<_>>()
                        .join(" → ")
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
        Err(e) => format!("invalid ({e})"),
    }
}
pub fn write(track: &mut Track, curve: &Curve) -> Result<(), String> {
    let mut curves = read(track)?;
    curves.retain(|c| c.target.layer != curve.target.layer);
    if !curve.points.is_empty() {
        curves.push(curve.clone());
    }
    let mut next = track.clone();
    next.events.retain(|e| !owned(e));
    if !curves.is_empty() {
        let mut data = PREFIX.to_vec();
        data.extend(encode(&curves)?);
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
    read(&next)?;
    expand(&next)?;
    refresh_deltas(&mut next)?;
    *track = next;
    Ok(())
}
/// Every segment blends only its two endpoint snapshots, even when skipping indices.
pub fn expand(track: &Track) -> Result<Vec<Curve>, String> {
    let curves = read(track)?;
    if curves.is_empty() {
        return Ok(vec![]);
    }
    let saved = super::macro_values::read(track)?;
    let mut result = vec![];
    for c in curves {
        let set = saved
            .variations
            .iter()
            .find(|s| s.layer == c.target.layer)
            .ok_or("Missing macro variations for curve")?;
        if c.points
            .iter()
            .any(|p| p.value as usize >= set.snapshots.len())
        {
            return Err("Macro variation curve references a missing variation".into());
        }
        for (parameter, template) in set.snapshots[0].iter().enumerate() {
            if format!(
                "{}:{}",
                template.target.plugin_format, template.target.plugin_id
            ) != c.target.plugin_id
            {
                return Err("Macro variation curve instrument changed".into());
            }
            let value = |p: &Point| set.snapshots[p.value as usize][parameter].points[0].value;
            let mut lane = template.clone();
            lane.points = c
                .points
                .iter()
                .map(|p| Point {
                    tick: p.tick,
                    value: value(p),
                })
                .collect();
            lane.controls = c
                .points
                .windows(2)
                .zip(&c.controls)
                .map(|(pair, control)| {
                    let (a, b) = (pair[0].value, pair[1].value);
                    let mix = if a == b {
                        0.0
                    } else {
                        (control.value - a) / (b - a)
                    };
                    Point {
                        tick: control.tick,
                        value: value(&pair[0]) * (1.0 - mix) + value(&pair[1]) * mix,
                    }
                })
                .collect();
            lane.validate()?;
            result.push(lane);
        }
    }
    Ok(result)
}
pub fn retain_layers(track: &mut Track, layers: &[u64]) -> Result<(), String> {
    let mut curves = read(track)?;
    let previous = curves.len();
    curves.retain(|c| layers.contains(&c.target.layer));
    if curves.len() == previous {
        return Ok(());
    }
    let data = encode(&curves)?;
    track.events.retain(|e| !owned(e));
    if !curves.is_empty() {
        let mut payload = PREFIX.to_vec();
        payload.extend(data);
        track.events.insert(
            0,
            Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 0x7f,
                data: payload,
            },
        );
    }
    refresh_deltas(track)
}
