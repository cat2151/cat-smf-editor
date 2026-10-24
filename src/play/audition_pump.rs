//! Service native work without leaving invalid bindings for the next GUI frame.
use super::Audition;
use std::sync::atomic::Ordering;

impl Audition {
    pub fn pump(&mut self) {
        if !self.needs_pump() {
            return;
        }
        self.poll_voices();
        self.pump_with(cat_smf_editor::host::Host::main_thread_work_pending());
    }

    // Diagnostics may write a log. Keep them outside every voice's short
    // notification/rebinding boundary, including the live-key voice.
    fn poll_voices(&mut self) {
        if let Some(voice) = &mut self.voice {
            voice.poll_diagnostics();
        }
        self.expire_cached(std::time::Instant::now());
        if let Some(voice) = &mut self.key_voice {
            voice.poll_voices();
        }
    }

    fn pump_with(&mut self, native_work: bool) {
        let key_voice = &mut self.key_voice;
        if let Some(voice) = &mut self.voice {
            if let Err(error) = voice.pump(native_work, || {
                if let Some(voice) = key_voice {
                    voice.pump_with(native_work);
                }
            }) {
                self.voice = None;
                self.progress = None;
                self.error = Some(format!("Audio automation refresh: {error}"));
                return;
            }
        } else if let Some(voice) = key_voice {
            voice.pump_with(native_work);
        }
        if let Some(voice) = &mut self.voice
            && voice.failed.load(Ordering::Relaxed)
        {
            match voice.refresh_automation() {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    self.voice = None;
                    self.progress = None;
                    self.error = Some(format!("Audio automation refresh: {error}"));
                    return;
                }
            }
            let error = voice.parameter_error().map(|code| {
                format!(
                    "Audio parameter batch failed ({code}): {}",
                    voice
                        .parameter_context
                        .as_deref()
                        .unwrap_or("unknown target")
                )
            });
            self.voice = None;
            self.progress = None;
            self.error =
                Some(error.unwrap_or_else(|| "Audio: plugin or audio device stopped".into()));
        }
    }
}
