//! Independent instruments and one post-sum effect chain, owned on the main thread.
use cat_smf_editor::{
    chain::Chain,
    chain_settings::Settings,
    master_effects,
    score::layer_track::{same_except_bypass, Patch},
};

pub struct Native {
    pub automated: bool,
    automated_layers: Vec<(usize, u64)>,
    layers: Vec<Chain>,
    effects: Option<Chain>,
    settings: Patch,
}

impl Native {
    #[cfg(test)]
    pub fn patch(&self) -> &Patch {
        &self.settings
    }
    pub fn parameters(
        &self,
        layer: u64,
    ) -> Result<Vec<cat_smf_editor::host::parameters::Parameter>, String> {
        let index = self
            .settings
            .layers
            .iter()
            .position(|l| l.settings.id == layer)
            .ok_or("Missing instrument layer")?;
        self.layers[index].parameters()
    }
    pub fn parameter_text(
        &self,
        layer: u64,
        value: cat_smf_editor::host::parameters::ParameterValue,
    ) -> Result<String, String> {
        let index = self
            .settings
            .layers
            .iter()
            .position(|l| l.settings.id == layer)
            .ok_or("Missing instrument layer")?;
        self.layers[index].parameter_text(value)
    }
    pub fn open(rate: u32, patch: &Patch) -> Result<Self, String> {
        patch.validate()?;
        let layers = patch
            .layers
            .iter()
            .map(|layer| {
                Chain::open(
                    rate,
                    &Settings {
                        instrument: layer.instrument.clone(),
                        ..Default::default()
                    },
                )
            })
            .collect::<Result<_, _>>()?;
        let effects = if patch.effects.is_empty() {
            None
        } else {
            Some(open_effects(rate, patch)?)
        };
        Ok(Self {
            automated: false,
            automated_layers: Vec::new(),
            layers,
            effects,
            settings: patch.clone(),
        })
    }

    /// Bypass changes reach the running chain without reopening it.
    pub fn set_bypass(&mut self, patch: &Patch) -> Result<(), String> {
        if !self.settings.same_except_bypass(patch) {
            return Err("Bypass change must keep the chain".into());
        }
        if let Some(effects) = &self.effects {
            effects.set_master_bypass(&patch.effects)?;
        }
        self.settings = patch.clone();
        Ok(())
    }

    pub fn reload(self, rate: u32, patch: &Patch) -> Result<Self, String> {
        patch.validate()?;
        let previous = self.restore_baseline(rate)?;
        let mut old: Vec<_> = previous
            .settings
            .layers
            .into_iter()
            .zip(previous.layers)
            .collect();
        let mut layers = Vec::new();
        for layer in &patch.layers {
            let desired = Settings {
                instrument: layer.instrument.clone(),
                ..Default::default()
            };
            let reusable = old.iter().position(|(previous, _)| {
                previous.settings.id == layer.settings.id
                    && (previous.instrument == layer.instrument
                        || Settings {
                            instrument: previous.instrument.clone(),
                            ..Default::default()
                        }
                        .same_plugins(&desired))
            });
            let chain = if let Some(index) = reusable {
                let (previous, chain) = old.swap_remove(index);
                if previous.instrument == layer.instrument {
                    chain
                } else {
                    chain.reload(rate, &desired)?
                }
            } else {
                Chain::open(rate, &desired)?
            };
            layers.push(chain);
        }
        let effects = match previous.effects {
            Some(effects) if same_except_bypass(&previous.settings.effects, &patch.effects) => {
                effects.set_master_bypass(&patch.effects)?;
                Some(effects)
            }
            _ if patch.effects.is_empty() => None,
            _ => Some(open_effects(rate, patch)?),
        };
        Ok(Self {
            automated: false,
            automated_layers: Vec::new(),
            layers,
            effects,
            settings: patch.clone(),
        })
    }

    pub fn pump(&self) {
        for layer in &self.layers {
            layer.pump();
        }
        if let Some(effects) = &self.effects {
            effects.pump();
        }
    }

    pub fn automation_changed(&self) -> bool {
        self.automated_layers
            .iter()
            .any(|&(layer, revision)| self.layers[layer].parameter_list_changes() != revision)
    }

    pub fn prepare_automation(
        &mut self,
        plan: &super::automation::OwnerPlan,
    ) -> Result<Vec<super::automation::LayerBindings>, String> {
        use super::automation::{Binding, LayerBindings};
        use cat_smf_editor::score::automation::{validate_parameter, ParameterSpec};
        let mut result: Vec<LayerBindings> = Vec::new();
        for curve in &plan.curves {
            let layer = self
                .settings
                .layers
                .iter()
                .position(|l| l.settings.id == curve.target.layer)
                .ok_or_else(|| format!("Missing automation layer: {}", curve.name))?;
            let key = match &self.settings.layers[layer].instrument {
                Some(slot) => slot.plugin.clone(),
                None => cat_smf_editor::chain::default_instrument()?,
            };
            if key.format != curve.target.plugin_format || key.id != curve.target.plugin_id {
                return Err(format!("Automation instrument changed: {}", curve.name));
            }
            let parameters = self.layers[layer].parameters()?;
            let mut matching = parameters
                .iter()
                .filter(|p| p.stable_id == curve.target.parameter_id);
            let parameter = matching
                .next()
                .ok_or_else(|| format!("Missing parameter: {} / {}", curve.path, curve.name))?;
            if matching.next().is_some() {
                return Err(format!("Ambiguous parameter ID: {}", curve.name));
            }
            validate_parameter(
                curve,
                ParameterSpec {
                    stable_id: &parameter.stable_id,
                    minimum: parameter.minimum,
                    maximum: parameter.maximum,
                    normalized: parameter.normalized,
                    eligible: parameter.eligible,
                },
            )?;
            let index = if let Some(i) = result.iter().position(|b| b.layer == layer) {
                i
            } else {
                self.layers[layer].prepare_parameters()?;
                result.push(LayerBindings {
                    layer,
                    lanes: Vec::new(),
                    values: Vec::new(),
                });
                result.len() - 1
            };
            if result[index].lanes.len() >= 256 {
                return Err("Too many automated parameters on one layer".into());
            }
            result[index].values.push(parameter.binding);
            result[index].lanes.push(Binding {
                live: plan.live.get(&curve.target).cloned(),
                curve: curve.clone(),
                value: parameter.binding,
            });
        }
        self.automated = !result.is_empty();
        self.automated_layers = result
            .iter()
            .map(|b| (b.layer, self.layers[b.layer].parameter_list_changes()))
            .collect();
        Ok(result)
    }

    /// UI thread, with every renderer stopped; use saved baseline bytes only.
    pub fn restore_baseline(mut self, rate: u32) -> Result<Self, String> {
        if self.automated {
            self.layers = std::mem::take(&mut self.layers)
                .into_iter()
                .enumerate()
                .zip(&self.settings.layers)
                .map(|((index, chain), layer)| {
                    if !self
                        .automated_layers
                        .iter()
                        .any(|&(layer, _)| layer == index)
                    {
                        return Ok(chain);
                    }
                    let desired = Settings {
                        instrument: layer.instrument.clone(),
                        ..Default::default()
                    };
                    if layer.instrument.is_some() {
                        chain.reload(rate, &desired)
                    } else {
                        drop(chain);
                        Chain::open(rate, &desired)
                    }
                })
                .collect::<Result<_, _>>()?;
            self.automated = false;
            self.automated_layers.clear();
        }
        Ok(self)
    }

    /// # Safety
    /// The returned renderer must stop before this Native is dropped.
    pub unsafe fn render(&self) -> super::layer_render::Render {
        let layers = self
            .layers
            .iter()
            .zip(&self.settings.layers)
            .map(|(chain, layer)| {
                (
                    cat_smf_editor::note_render::Render::new(unsafe { chain.audio_ref() }),
                    layer.settings.clone(),
                )
            })
            .collect();
        super::layer_render::Render::new(
            layers,
            self.effects
                .as_ref()
                .map(|chain| unsafe { chain.audio_ref() }),
        )
    }

    pub fn silence(&self, out: &mut [f32]) -> bool {
        self.layers
            .iter()
            .all(|chain| unsafe { chain.audio_ref() }.process(&[0x20b0_7800], out, 2))
    }
}

fn open_effects(rate: u32, patch: &Patch) -> Result<Chain, String> {
    Chain::open_master(
        rate,
        &master_effects::Settings {
            effects: patch.effects.clone(),
            ..Default::default()
        },
    )
}

#[cfg(test)]
#[path = "layer_native_tests.rs"]
mod tests;
