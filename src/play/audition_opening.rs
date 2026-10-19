//! Frame-by-frame audio setup for an opened document; all plugins stay on the UI thread.
use super::{output, Audition};
use cat_smf_editor::{master_effects, score::instrument::Patch};

pub struct Preparation(Option<output::Builder>);

impl Preparation {
    pub fn new(audition: &Audition) -> Result<Self, String> {
        Ok(Self(if audition.is_disabled() {
            None
        } else {
            Some(output::Builder::new(None)?)
        }))
    }

    pub fn track(&mut self, track: usize, patch: &Patch) -> Result<(), String> {
        if let Some(builder) = &mut self.0 {
            builder.track(track, patch)?;
        }
        Ok(())
    }

    pub fn master(&mut self, settings: &master_effects::Settings) -> Result<(), String> {
        if let Some(builder) = &mut self.0 {
            builder.master(settings)?;
        }
        Ok(())
    }

    pub fn pump(&self) {
        if let Some(builder) = &self.0 {
            builder.pump();
        }
    }

    pub fn finish(self, audition: &mut Audition) -> Result<(), String> {
        if let Some(builder) = self.0 {
            audition.voice = Some(builder.finish(!audition.auto_gain_enabled())?);
            audition.last_browser_use = Some(std::time::Instant::now());
        }
        Ok(())
    }
}
