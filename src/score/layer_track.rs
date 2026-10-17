//! Runtime track settings. Only `saved()` crosses the SMF persistence boundary.
use crate::{
    chain_settings::{PluginState, Settings as Chain},
    instrument_layers::{self, Layer},
    patch_reference::PatchReference,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedLayer {
    pub settings: Layer,
    pub instrument: Option<PluginState>,
}

impl ResolvedLayer {
    pub fn label(&self) -> &str {
        self.instrument.as_ref().map_or(DEFAULT_LABEL, |s| {
            s.preset_name.as_deref().unwrap_or(&s.plugin.name)
        })
    }
}

/// The sound of a layer without a patch.
pub const DEFAULT_LABEL: &str = "Surge XT / Init Saw";

#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    pub layers: Vec<ResolvedLayer>,
    pub effects: Vec<PluginState>,
}

impl Default for Patch {
    fn default() -> Self {
        Self {
            layers: vec![ResolvedLayer {
                settings: Layer::default(),
                instrument: None,
            }],
            effects: vec![],
        }
    }
}

pub fn reference(slot: &PluginState) -> Result<PatchReference, String> {
    let reference = slot
        .reference
        .as_ref()
        .ok_or("Plugin state has no restoration reference")?;
    reference.validate()?;
    if reference.plugin != slot.plugin {
        return Err("State/reference plugin mismatch".into());
    }
    Ok(reference.clone())
}

/// Saved form of one chained effect.
pub fn effect_reference(slot: &PluginState) -> Result<instrument_layers::Effect, String> {
    Ok(instrument_layers::Effect {
        patch: reference(slot)?,
        bypass: slot.bypass,
    })
}

/// Resolves a saved effect, carrying its bypass onto the state.
pub fn resolve_effect(
    effect: &instrument_layers::Effect,
    resolve: impl FnOnce(&PatchReference) -> Result<PluginState, String>,
) -> Result<PluginState, String> {
    Ok(PluginState {
        bypass: effect.bypass,
        ..resolve(&effect.patch)?
    })
}

/// Same plugins and states in the same order; only bypass may differ.
pub fn same_except_bypass(a: &[PluginState], b: &[PluginState]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            PluginState {
                bypass: b.bypass,
                ..a.clone()
            } == *b
        })
}

impl Patch {
    pub fn resolve(
        saved: instrument_layers::Settings,
        mut resolve: impl FnMut(&PatchReference) -> Result<PluginState, String>,
    ) -> Result<Self, String> {
        saved.validate()?;
        let layers = saved
            .layers
            .into_iter()
            .map(|settings| {
                let instrument = settings.patch.as_ref().map(&mut resolve).transpose()?;
                Ok(ResolvedLayer {
                    settings,
                    instrument,
                })
            })
            .collect::<Result<_, String>>()?;
        let effects = saved
            .effects
            .iter()
            .map(|effect| resolve_effect(effect, &mut resolve))
            .collect::<Result<_, _>>()?;
        Ok(Self { layers, effects })
    }
    pub fn saved(&self) -> Result<instrument_layers::Settings, String> {
        let settings = instrument_layers::Settings {
            layers: self
                .layers
                .iter()
                .map(|layer| {
                    let mut settings = layer.settings.clone();
                    settings.patch = layer.instrument.as_ref().map(reference).transpose()?;
                    Ok(settings)
                })
                .collect::<Result<_, String>>()?,
            effects: self
                .effects
                .iter()
                .map(effect_reference)
                .collect::<Result<_, _>>()?,
            ..Default::default()
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.saved()?;
        for layer in &self.layers {
            Chain {
                instrument: layer.instrument.clone(),
                ..Default::default()
            }
            .validate()?;
        }
        Chain {
            effects: self.effects.clone(),
            ..Default::default()
        }
        .validate_effect_states()
    }

    pub fn label(&self) -> &str {
        self.layers.first().map_or("—", ResolvedLayer::label)
    }

    /// `other` differs from `self` at most in effect bypass.
    pub fn same_except_bypass(&self, other: &Self) -> bool {
        self.layers == other.layers && same_except_bypass(&self.effects, &other.effects)
    }

    pub fn same_plugins(&self, other: &Self) -> bool {
        self.layers.iter().any(|a| {
            other.layers.iter().any(|b| {
                a.settings.id == b.settings.id
                    && (a.instrument == b.instrument
                        || Chain {
                            instrument: a.instrument.clone(),
                            ..Default::default()
                        }
                        .same_plugins(&Chain {
                            instrument: b.instrument.clone(),
                            ..Default::default()
                        }))
            })
        }) && self
            .effects
            .iter()
            .map(|e| &e.plugin)
            .eq(other.effects.iter().map(|e| &e.plugin))
    }
}

impl From<Chain> for Patch {
    fn from(chain: Chain) -> Self {
        Self {
            layers: vec![ResolvedLayer {
                settings: Layer::default(),
                instrument: chain.instrument,
            }],
            effects: chain
                .effects
                .into_iter()
                .map(|effect| PluginState {
                    bypass: effect.bypass || chain.bypass,
                    ..effect
                })
                .collect(),
        }
    }
}

pub fn write(
    track: &mut crate::model::Track,
    patch: &Patch,
    roots: &mut dyn crate::patch_roots::Roots,
) -> Result<(), String> {
    instrument_layers::write(track, &patch.saved()?, roots)
}
