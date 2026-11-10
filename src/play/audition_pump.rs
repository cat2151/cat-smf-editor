//! Service native work without leaving invalid bindings for the next GUI frame.
use super::Audition;
use cat_smf_editor::message;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Repaints while a stream renders, so its diagnostics and plugin requests keep moving.
const RENDERING: Duration = Duration::from_millis(32);
/// Repaints while every stream rests: only plugins' main-thread requests need servicing.
const RESTING: Duration = Duration::from_millis(500);

fn interval(idle: bool) -> Duration {
    if idle {
        RESTING
    } else {
        RENDERING
    }
}

impl Audition {
    /// How soon the next frame must pump; `None` without any stream.
    pub fn pump_interval(&self) -> Option<Duration> {
        self.needs_pump().then(|| interval(self.idle()))
    }

    /// Whether every open stream skips its plugins and none of their plugins waits for
    /// main-thread work; true without any stream.
    pub fn idle(&self) -> bool {
        self.resting()
            && (!self.needs_pump() || !cat_smf_editor::host::Host::main_thread_work_pending())
    }

    fn resting(&self) -> bool {
        self.voice.as_ref().is_none_or(|voice| voice.resting())
            && [&self.key_voice, &self.live_voice]
                .into_iter()
                .flatten()
                .all(|voice| voice.resting())
    }

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
        for voice in self.side_voices() {
            voice.poll_voices();
        }
    }

    fn pump_with(&mut self, native_work: bool) {
        let side = [&mut self.key_voice, &mut self.live_voice];
        let pump_side = |side: [&mut Option<Box<Self>>; 2]| {
            for voice in side.into_iter().flatten() {
                voice.pump_with(native_work);
            }
        };
        if let Some(voice) = &mut self.voice {
            if let Err(error) = voice.pump(native_work, || pump_side(side)) {
                self.voice = None;
                self.progress = None;
                self.error = Some(message::audio_automation_refresh(error));
                return;
            }
        } else {
            pump_side(side);
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
                    self.error = Some(message::audio_automation_refresh(error));
                    return;
                }
            }
            let error = voice.parameter_error().map(|code| {
                format!(
                    "音声: parameterの一括設定に失敗しました ({code}): {}",
                    voice.parameter_context.as_deref().unwrap_or("不明な対象")
                )
            });
            self.voice = None;
            self.progress = None;
            self.error =
                Some(error.unwrap_or_else(|| "音声: pluginか音声デバイスが止まりました".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn resting_streams_pump_far_less_often_than_rendering_ones() {
        assert_eq!(super::interval(false), super::RENDERING);
        assert_eq!(super::interval(true), super::RESTING);
        assert!(super::RESTING >= super::RENDERING * 10);
    }
}
