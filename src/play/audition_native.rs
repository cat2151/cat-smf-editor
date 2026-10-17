//! Independent instruments and one post-sum effect chain, owned on the main thread.
use cat_smf_editor::{
    chain::Chain,
    chain_settings::Settings,
    master_effects,
    score::layer_track::{same_except_bypass, Patch},
};

pub struct Native {
    layers: Vec<Chain>,
    effects: Option<Chain>,
    settings: Patch,
}

impl Native {
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
        let mut old: Vec<_> = self.settings.layers.into_iter().zip(self.layers).collect();
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
        let effects = match self.effects {
            Some(effects) if same_except_bypass(&self.settings.effects, &patch.effects) => {
                effects.set_master_bypass(&patch.effects)?;
                Some(effects)
            }
            _ if patch.effects.is_empty() => None,
            _ => Some(open_effects(rate, patch)?),
        };
        Ok(Self {
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
