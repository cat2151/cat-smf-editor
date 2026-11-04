//! CC1 sweeps only while the keys are recorded, so an idle keyboard holds its modulation.
use super::*;
use std::time::Duration;

impl Session {
    /// Where the CC1 written for the keys starts its sweep; until the sweep starts, after
    /// every key so far.
    pub(super) fn sweep_start(&self) -> Duration {
        self.sweep.unwrap_or_else(|| self.clock.elapsed())
    }
    /// The CC1 sent at `at`; the sweep waits at its start until the keys are recorded.
    pub(super) fn cc1(&self, at: Duration) -> Option<u8> {
        let since = self.sweep.map_or(Duration::ZERO, |z| at.saturating_sub(z));
        self.settings().modulation.value(since)
    }
}

/// The keys are recorded: a take is armed, a phrase loop captures, or a Capture phrase goes on.
fn recording(s: &Session, at: Duration) -> bool {
    super::play::armed(s)
        || s.sounds
            .iter()
            .any(|x| matches!(x.looper, super::phrase::Looper::Capturing { .. }))
        || super::capture::playing(s, at)
}

/// The CC1 sweep runs from `at` while the keys are recorded and waits at its start otherwise.
pub(super) fn follow(s: &mut Session, at: Duration) {
    s.sweep = recording(s, at).then(|| s.sweep.unwrap_or(at));
    if let Some(q) = s.queue() {
        let sweep = s.sweep.map(|z| s.clock + z);
        q.lock().unwrap().modulate(sweep, s.settings().modulation);
    }
}
