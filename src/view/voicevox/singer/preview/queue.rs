//! Waiting reservations only; the running render is owned by the worker.
use cat_smf_editor::voicevox::Voice;

#[derive(Default)]
pub(super) struct Queue {
    voices: Vec<Voice>,
}
impl Queue {
    pub fn reserve(&mut self, voice: Voice) {
        if !self.voices.iter().any(|v| v.same_style(&voice)) {
            self.voices.push(voice);
        }
    }
    pub fn cancel(&mut self, voice: &Voice) {
        self.voices.retain(|v| !v.same_style(voice));
    }
    pub fn front(&mut self, voice: Voice) {
        self.cancel(&voice);
        self.voices.insert(0, voice);
    }
    pub fn deduplicate(&mut self) {
        let mut seen: Vec<Voice> = Vec::new();
        self.voices.retain(|v| {
            if seen.iter().any(|s| s.same_style(v)) {
                false
            } else {
                seen.push(v.clone());
                true
            }
        });
    }
    pub fn sort(&mut self, selected: Option<&Voice>, visible: &[Voice], catalog: &[Voice]) {
        self.voices.sort_by_key(|v| {
            let priority = if selected.is_some_and(|s| s.same_style(v)) {
                0
            } else if selected.is_some_and(|s| s.speaker_uuid == v.speaker_uuid) {
                1
            } else if visible.iter().any(|s| s.same_style(v)) {
                2
            } else {
                3
            };
            (
                priority,
                catalog
                    .iter()
                    .position(|c| c.same_style(v))
                    .unwrap_or(usize::MAX),
            )
        });
    }
    pub fn retain_catalog(&mut self, catalog: &[Voice]) {
        self.voices
            .retain(|v| catalog.iter().any(|c| c.same_style(v)));
    }
    pub fn pop(&mut self) -> Option<Voice> {
        if self.voices.is_empty() {
            None
        } else {
            Some(self.voices.remove(0))
        }
    }
    pub fn clear(&mut self) {
        self.voices.clear();
    }
    #[cfg(test)]
    pub fn snapshot(&self) -> Vec<Voice> {
        self.voices.clone()
    }
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }
}

#[cfg(test)]
mod tests;
