//! Fixed track-head values, separate from automation tracks and patch state caches.
use crate::{
    edit::{parameter_curve::Curve, refresh_deltas},
    model::{Event, Track},
};
use serde::{Deserialize, Serialize};
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0macro-values\0";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Values {
    pub version: u8,
    pub parameters: Vec<Curve>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variations: Vec<Variations>,
}
/// Editable alternatives for one instrument layer; parameters holds the active choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variations {
    pub layer: u64,
    pub selected: usize,
    pub snapshots: Vec<Vec<Curve>>,
}
fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 0x7f && e.data.starts_with(PREFIX)
}
impl Values {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.parameters.len() > 512 {
            return Err("Invalid macro values".into());
        }
        if self.variations.len() > 64 {
            return Err("Too many macro variation sets".into());
        }
        let mut layers = std::collections::HashSet::new();
        for set in &self.variations {
            if !layers.insert(set.layer)
                || set.snapshots.is_empty()
                || set.snapshots.len() > 128
                || set.selected >= set.snapshots.len()
            {
                return Err("Invalid macro variations".into());
            }
            let active: Vec<_> = self
                .parameters
                .iter()
                .filter(|c| c.target.layer == set.layer)
                .cloned()
                .collect();
            if active != set.snapshots[set.selected] {
                return Err("Macro variation differs from active values".into());
            }
            for snapshot in &set.snapshots {
                if snapshot.is_empty()
                    || snapshot.len() != active.len()
                    || snapshot
                        .iter()
                        .zip(&active)
                        .any(|(a, b)| a.target != b.target || a.value_space != b.value_space)
                {
                    return Err("Macro variation targets differ".into());
                }
                validate_parameters(snapshot)?;
            }
        }
        validate_parameters(&self.parameters)
    }
}
fn validate_parameters(parameters: &[Curve]) -> Result<(), String> {
    let mut targets = std::collections::HashSet::new();
    for c in parameters {
        c.validate()?;
        let range =
            crate::instrument_macros::Instrument::of(&c.target.plugin_format, &c.target.plugin_id)
                .map(|i| i.range());
        if range != Some((c.minimum, c.maximum))
            || c.target.group != 1
            || c.points.len() != 1
            || c.points[0].tick != 0
            || !targets.insert(c.target.clone())
        {
            return Err("Invalid or duplicate fixed macro target".into());
        }
    }
    Ok(())
}
pub fn read(track: &Track) -> Result<Values, String> {
    let mut events = track.events.iter().filter(|e| owned(e));
    let Some(e) = events.next() else {
        return Ok(Values {
            version: 1,
            parameters: vec![],
            variations: vec![],
        });
    };
    if e.tick != 0 || e.data.len() > 1024 * 1024 || events.next().is_some() {
        return Err("Misplaced or duplicate macro values".into());
    }
    let values: Values =
        serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
    values.validate()?;
    Ok(values)
}
pub fn write(track: &mut Track, values: &Values) -> Result<(), String> {
    read(track)?;
    values.validate()?;
    let mut next = track.clone();
    next.events.retain(|e| !owned(e));
    if !values.parameters.is_empty() {
        let mut data = PREFIX.to_vec();
        data.extend(serde_json::to_vec(values).map_err(|e| e.to_string())?);
        if data.len() > 1024 * 1024 {
            return Err("Macro values are too large".into());
        }
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
    refresh_deltas(&mut next)?;
    *track = next;
    Ok(())
}

pub fn retain_unchanged_layers(
    track: &mut Track,
    before: &super::instrument::Patch,
    after: &super::instrument::Patch,
) -> Result<(), String> {
    let mut values = read(track)?;
    values.parameters.retain(|c| {
        after.layers.iter().any(|next| {
            next.settings.id == c.target.layer
                && before.layers.iter().any(|old| {
                    old.settings.id == next.settings.id && old.instrument == next.instrument
                })
        })
    });
    values.variations.retain(|set| {
        values
            .parameters
            .iter()
            .any(|c| c.target.layer == set.layer)
    });
    super::macro_curve::retain_layers(
        track,
        &values
            .variations
            .iter()
            .map(|s| s.layer)
            .collect::<Vec<_>>(),
    )?;
    write(track, &values)
}
