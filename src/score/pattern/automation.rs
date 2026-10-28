//! Pattern-relative curves and immutable container baselines, separate from runtime values.
use super::*;
use crate::edit::parameter_curve::{Curve, Target, ValueSpace};
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0pattern-parameter\0";
pub const BASELINES: &[u8] = b"\x7dcat-smf-editor\0container-baselines\0";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Baseline {
    Parameter {
        target: Target,
        value_space: ValueSpace,
        minimum: f64,
        maximum: f64,
        value: f64,
    },
    Midi {
        channel: u8,
        controller: Option<u8>,
        value: u16,
    },
}
fn meta(e: &Event, prefix: &[u8]) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(prefix)
}
pub fn owned(e: &Event) -> bool {
    meta(e, PREFIX)
}
fn event<T: Serialize>(tick: u64, prefix: &[u8], value: &T) -> Result<Event, String> {
    Ok(Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x7f,
        data: [
            prefix,
            serde_json::to_vec(value)
                .map_err(|e| e.to_string())?
                .as_slice(),
        ]
        .concat(),
    })
}
pub fn baselines(m: &Music, track: usize) -> Result<Vec<Baseline>, String> {
    let t = m.tracks.get(track).ok_or("Missing container")?;
    let records = t
        .events
        .iter()
        .filter(|e| meta(e, BASELINES))
        .collect::<Vec<_>>();
    if records.len() > 1 || records.first().is_some_and(|e| e.tick != 0) {
        return Err("Invalid container baselines".into());
    }
    let values = records
        .first()
        .map(|e| {
            serde_json::from_slice::<Vec<Baseline>>(&e.data[BASELINES.len()..])
                .map_err(|e| e.to_string())
        })
        .transpose()?
        .unwrap_or_default();
    for (i, b) in values.iter().enumerate() {
        match b {
            Baseline::Parameter {target,value_space,minimum,maximum,value} => {
                crate::host::parameters::domain::Domain::new(*value_space,*minimum,*maximum)?;
                if !value.is_finite() || !(*minimum..=*maximum).contains(value) || values[..i].iter().any(|other|matches!(other,Baseline::Parameter {target:t,..} if t==target)) {
                    return Err("Invalid or duplicate parameter baseline".into());
                }
            }
            Baseline::Midi {channel,controller,value} => {
                if *channel>15 || controller.is_some_and(|cc|!supported_cc(cc)) || *value>if controller.is_some(){127}else{16383}
                    || values[..i].iter().any(|other|matches!(other,Baseline::Midi {channel:c,controller:cc,..} if c==channel && cc==controller)) {
                    return Err("Invalid or duplicate MIDI baseline".into());
                }
            }
        }
    }
    Ok(values)
}
pub fn supported_cc(cc: u8) -> bool {
    !matches!(cc,0|6|32|38|96..=101|120..=127) && cc < 128
}
pub fn curves(m: &Music, p: &Definition) -> Result<Vec<Curve>, String> {
    let mut out = vec![];
    for e in &m.tracks[p.track].events {
        if !owned(e) || !(p.tick..=p.range()?.1).contains(&e.tick) {
            continue;
        }
        if e.tick != p.tick {
            return Err("Pattern curves must be stored at the body start".into());
        }
        let c: Curve =
            serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
        c.validate()?;
        if c.target.group != p.container
            || c.points
                .iter()
                .chain(&c.controls)
                .any(|point| point.tick > p.length().unwrap_or(0))
            || out.iter().any(|other: &Curve| other.target == c.target)
        {
            return Err("Invalid pattern curve ownership or range".into());
        }
        out.push(c);
    }
    Ok(out)
}
pub fn write(
    m: &mut Music,
    id: u64,
    mut curve: Curve,
    baseline: Option<f64>,
) -> Result<(), String> {
    let catalog = Catalog::read(m)?;
    let p = catalog.patterns.get(&id).ok_or("Missing pattern")?;
    curve.target.group = p.container;
    curve.validate()?;
    let mut values = baselines(m, p.track)?;
    if !values
        .iter()
        .any(|b| matches!(b,Baseline::Parameter {target,..} if target==&curve.target))
    {
        let value = baseline.ok_or("Missing parameter baseline")?;
        values.push(Baseline::Parameter {
            target: curve.target.clone(),
            value_space: curve.value_space,
            minimum: curve.minimum,
            maximum: curve.maximum,
            value,
        });
    }
    let mut next = m.clone();
    let t = &mut next.tracks[p.track];
    t.events.retain(|e| {
        !meta(e, BASELINES)
            && !(owned(e)
                && e.tick == p.tick
                && serde_json::from_slice::<Curve>(&e.data[PREFIX.len()..])
                    .is_ok_and(|c| c.target == curve.target))
    });
    crate::edit::insert(t, event(0, BASELINES, &values)?);
    crate::edit::insert(t, event(p.tick, PREFIX, &curve)?);
    crate::edit::refresh_deltas(t)?;
    curves(&next, p)?;
    baselines(&next, p.track)?;
    Catalog::read(&next)?;
    *m = next;
    Ok(())
}
pub fn set_midi_baseline(
    m: &mut Music,
    container: u64,
    channel: u8,
    controller: Option<u8>,
    value: u16,
) -> Result<(), String> {
    let catalog = Catalog::read(m)?;
    let track = *catalog
        .tracks
        .get(&container)
        .filter(|t| catalog.sources.contains(t))
        .ok_or("Missing container")?;
    let mut values = baselines(m, track)?;
    values.retain(|b|!matches!(b,Baseline::Midi {channel:c,controller:cc,..} if *c==channel && *cc==controller));
    values.push(Baseline::Midi {
        channel,
        controller,
        value,
    });
    let mut next = m.clone();
    let t = &mut next.tracks[track];
    t.events.retain(|e| !meta(e, BASELINES));
    crate::edit::insert(t, event(0, BASELINES, &values)?);
    crate::edit::refresh_deltas(t)?;
    baselines(&next, track)?;
    Catalog::read(&next)?;
    *m = next;
    Ok(())
}
pub fn describe(data: &[u8]) -> String {
    serde_json::from_slice::<Curve>(&data[PREFIX.len()..])
        .map(|c| format!("{} · {} · {} points", c.path, c.name, c.points.len()))
        .unwrap_or_else(|_| "invalid".into())
}
pub fn describe_baselines(data: &[u8]) -> String {
    serde_json::from_slice::<Vec<Baseline>>(&data[BASELINES.len()..])
        .map(|values| {
            values
                .iter()
                .map(|b| match b {
                    Baseline::Parameter {
                        target,
                        value_space,
                        minimum,
                        maximum,
                        value,
                    } => format!(
                        "Layer {} / {}: {} ({value_space:?} {minimum}..{maximum})",
                        target.layer, target.parameter_id, value
                    ),
                    Baseline::Midi {
                        channel,
                        controller,
                        value,
                    } => format!(
                        "Channel {} / {}: {}",
                        channel + 1,
                        controller.map_or("Pitch bend".into(), |cc| format!("CC {cc}")),
                        value
                    ),
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_else(|_| "invalid".into())
}

pub fn parameter_baseline(
    m: &mut Music,
    container: u64,
    curve: &Curve,
    value: f64,
) -> Result<(), String> {
    let c = Catalog::read(m)?;
    let track = *c
        .tracks
        .get(&container)
        .filter(|t| c.sources.contains(t))
        .ok_or("Missing destination container")?;
    let mut target = curve.target.clone();
    target.group = container;
    let mut values = baselines(m, track)?;
    if values
        .iter()
        .any(|b| matches!(b,Baseline::Parameter {target:t,..} if t==&target))
    {
        return Ok(());
    }
    values.push(Baseline::Parameter {
        target,
        value_space: curve.value_space,
        minimum: curve.minimum,
        maximum: curve.maximum,
        value,
    });
    let mut next = m.clone();
    let t = &mut next.tracks[track];
    t.events.retain(|e| !meta(e, BASELINES));
    crate::edit::insert(t, event(0, BASELINES, &values)?);
    crate::edit::refresh_deltas(t)?;
    baselines(&next, track)?;
    *m = next;
    Ok(())
}
pub fn retarget(events: &mut [Event], container: u64) -> Result<(), String> {
    for e in events.iter_mut().filter(|e| owned(e)) {
        let mut c: Curve =
            serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
        c.target.group = container;
        *e = event(e.tick, PREFIX, &c)?;
    }
    Ok(())
}
