//! Main-thread notifications and automation rebinding share one render boundary.
use super::Voice;
use crate::play::layer_render::Render;
use std::sync::atomic::Ordering;

impl Voice {
    pub fn pump(&mut self, native_work: bool, other_voices: impl FnOnce()) -> Result<(), String> {
        if !self.automated() {
            other_voices();
            if native_work {
                self.pump_plugins();
            }
            return Ok(());
        }
        // All app-owned chains have no native editor windows. Defer newly
        // enqueued callbacks to the next poll instead of taking the audio lock
        // every GUI frame for an empty queue.
        if !native_work && !self.natives.iter().any(|n| n.automation_changed()) {
            other_voices();
            return Ok(());
        }
        // Pump drains a shared native event loop, including other hosts' work.
        // A rescan can replace parameter cookies before publishing its revision.
        // Exclude rendering throughout notification delivery and rebinding.
        let renders = self.renders.clone();
        let mut renders = renders.lock().map_err(|e| e.to_string())?;
        other_voices();
        if native_work {
            self.pump_plugins();
        }
        self.refresh_bindings(&mut renders)?;
        Ok(())
    }

    fn pump_plugins(&self) {
        self.pump_cached();
        for native in &self.natives {
            native.pump();
        }
        if let Some(master) = &self.master {
            master.pump();
        }
    }

    /// Recover an invalidation delivered outside the regular pump boundary.
    pub fn refresh_automation(&mut self) -> Result<bool, String> {
        let renders = self.renders.clone();
        let mut renders = renders.lock().map_err(|e| e.to_string())?;
        self.refresh_bindings(&mut renders)
    }

    fn refresh_bindings(&mut self, renders: &mut [Render]) -> Result<bool, String> {
        if renders
            .iter()
            .any(|r| r.parameter_error.is_some_and(|code| code != -6))
        {
            return Ok(false);
        }
        let command = self.mailbox.lock().map_err(|e| e.to_string())?;
        let mut recovered = false;
        let mut refreshed = false;
        for ((native, render), part) in self
            .natives
            .iter_mut()
            .zip(renders)
            .zip(command.parts.iter())
        {
            let invalid = render.parameter_error == Some(-6);
            if (invalid || native.automation_changed())
                && let Some(plan) = &part.automation
            {
                render.set_automation(native.prepare_automation(&plan.owner)?);
                recovered |= invalid;
                refreshed = true;
            }
        }
        // Proactive metadata refresh must not clear a device/render failure.
        if recovered {
            self.failed.store(false, Ordering::Relaxed);
        }
        Ok(refreshed)
    }
}
