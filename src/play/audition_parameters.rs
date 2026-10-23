//! UI-only discovery on the same owner instance used by sequence playback.
use super::Audition;
use cat_smf_editor::{
    host::parameters::{Parameter, ParameterValue},
    score::instrument::Patch,
};

impl Audition {
    pub fn has_parameter_owner(&self, owner: usize, patch: &Patch) -> bool {
        self.voice
            .as_ref()
            .is_some_and(|v| v.patches.iter().any(|(t, p)| *t == owner && p == patch))
    }
    pub fn parameter_catalog(
        &mut self,
        owner: usize,
        patch: &Patch,
        layer: u64,
    ) -> Result<Vec<Parameter>, String> {
        if !self.has_parameter_owner(owner, patch) && !self.prepare(&[(owner, patch.clone())]) {
            return Err(self
                .error()
                .unwrap_or("Parameter instrument unavailable")
                .into());
        }
        let voice = self
            .voice
            .as_ref()
            .ok_or("Parameter instrument unavailable")?;
        let index = voice
            .patches
            .iter()
            .position(|(t, p)| *t == owner && p == patch)
            .ok_or("Parameter owner changed")?;
        voice.natives[index].parameters(layer)
    }
    pub fn parameter_text(
        &self,
        owner: usize,
        layer: u64,
        value: ParameterValue,
    ) -> Result<String, String> {
        let voice = self
            .voice
            .as_ref()
            .ok_or("Parameter instrument unavailable")?;
        let index = voice
            .patches
            .iter()
            .position(|(t, _)| *t == owner)
            .ok_or("Parameter owner changed")?;
        voice.natives[index].parameter_text(layer, value)
    }
}
