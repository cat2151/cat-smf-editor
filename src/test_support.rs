use cat_smf_editor::{
    chain_settings::PluginState,
    patch_reference::{PatchReference, Source},
};
pub mod instrument;
pub fn referenced(mut slot: PluginState) -> PluginState {
    let path = std::path::PathBuf::from(format!("X:/patches/{}.preset", slot.plugin.id));
    slot.reference = Some(PatchReference {
        plugin: slot.plugin.clone(),
        source: Source::Catalog {
            path: path.clone(),
            file: path,
        },
        display: slot
            .preset_name
            .clone()
            .unwrap_or_else(|| slot.plugin.name.clone()),
    });
    slot
}

pub fn master_saved(
    settings: &cat_smf_editor::master_effects::Settings,
) -> cat_smf_editor::master_references::Effects {
    cat_smf_editor::master_references::Effects {
        effects: settings
            .effects
            .iter()
            .map(|e| cat_smf_editor::instrument_layers::Effect {
                patch: e.reference.clone().unwrap(),
                bypass: e.bypass,
            })
            .collect(),
    }
}
