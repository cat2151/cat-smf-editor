//! What in a session moves with time alone, so the app draws again without input.
use super::*;

impl Session {
    /// Something here changes with the clock rather than with input: a take counting in or
    /// recording, a phrase capturing or repeating, keys held, Capture keys not yet in the song,
    /// a CC1 sweep running, or a kit being created.
    pub(super) fn ticking(&self) -> bool {
        play::armed(self)
            || self.sounds.iter().any(|x| {
                matches!(
                    x.looper,
                    phrase::Looper::Capturing { .. } | phrase::Looper::Looping { .. }
                )
            })
            || self
                .recorder
                .as_ref()
                .is_some_and(|r| r.held_pads().next().is_some())
            || self.mouse.iter().any(Option::is_some)
            || capture::pending(self)
            || self.sweep.is_some()
            || self.kit_creation.is_some()
    }
}
