//! One owner/layer/plugin identity for playback and target presentation.
use crate::{
    edit::parameter_curve::{Collection, Curve, ValueSpace},
    host::PluginKey,
    score::layer_track::Patch,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub owner: usize,
    pub layer_index: usize,
}

pub fn resolve(
    collection: &Collection,
    track: usize,
    patches: &[(usize, Patch)],
    default_plugin: Option<&PluginKey>,
) -> Result<ResolvedTarget, String> {
    resolve_in(
        collection,
        track,
        patches.iter().map(|(i, p)| (*i, p)),
        default_plugin,
    )
}

/// Resolve saved identities without copying the owners' instrument states.
pub fn resolve_in<'a>(
    collection: &Collection,
    track: usize,
    patches: impl IntoIterator<Item = (usize, &'a Patch)>,
    default_plugin: Option<&PluginKey>,
) -> Result<ResolvedTarget, String> {
    let curve = collection
        .curves
        .get(&track)
        .ok_or("Not a parameter track")?;
    if collection
        .conflicts()
        .iter()
        .any(|tracks| tracks.contains(&track))
    {
        return Err(format!(
            "Conflicting parameter curves: {} / {}",
            curve.path, curve.name
        ));
    }
    let owner = collection
        .owner(track)
        .ok_or_else(|| format!("Missing owner for parameter {}", curve.name))?;
    let patch = patches
        .into_iter()
        .find_map(|(index, patch)| (index == owner).then_some(patch))
        .ok_or_else(|| format!("Owner track {} is unavailable", owner + 1))?;
    let layer_index = patch
        .layers
        .iter()
        .position(|layer| layer.settings.id == curve.target.layer)
        .ok_or_else(|| format!("Missing layer for parameter {}", curve.name))?;
    let layer = &patch.layers[layer_index];
    let plugin = layer
        .instrument
        .as_ref()
        .map(|slot| &slot.plugin)
        .or(default_plugin)
        .ok_or("Parameter instrument is unavailable")?;
    if plugin.format != curve.target.plugin_format || plugin.id != curve.target.plugin_id {
        return Err(format!("Instrument changed for parameter {}", curve.name));
    }
    Ok(ResolvedTarget { owner, layer_index })
}

pub struct ParameterSpec<'a> {
    pub stable_id: &'a str,
    pub minimum: f64,
    pub maximum: f64,
    pub normalized: bool,
    pub eligible: bool,
}

pub fn validate_parameter(curve: &Curve, parameter: ParameterSpec<'_>) -> Result<(), String> {
    curve.validate()?;
    if curve.target.parameter_id != parameter.stable_id || !parameter.eligible {
        return Err(format!(
            "Parameter is missing or not automatable: {} / {}",
            curve.path, curve.name
        ));
    }
    if parameter.normalized != (curve.value_space == ValueSpace::Normalized)
        || parameter.minimum != curve.minimum
        || parameter.maximum != curve.maximum
    {
        return Err(format!(
            "Parameter range or value space changed: {} / {}",
            curve.path, curve.name
        ));
    }
    Ok(())
}
