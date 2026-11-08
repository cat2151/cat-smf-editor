//! No file location, metadata, parsing or monitor enumeration on the first-audio path.
use crate::{app::App, play::transport};
use eframe::egui;

fn ready(a: &App, heard: bool) -> bool {
    !a.startup.pending
        && (heard
            || a.audition.is_disabled()
            || a.audition.error().is_some()
            || (!transport::preparing(a)
                && a.playback.active.is_none()
                && !crate::play::sung::prerender::running(a)))
}

pub(crate) fn update(a: &mut App, ctx: &egui::Context) {
    if !ready(a, cat_smf_editor::startup_trace::audio_started()) {
        ctx.request_repaint_after(super::POLL);
        return;
    }
    // UI fixtures install their own Spec and must never read the developer's live layout.
    #[cfg(not(test))]
    if !a.layout.watching()
        && let Some(directory) = crate::store::config::directory()
    {
        let _span = cat_smf_editor::startup_trace::span("layout_read");
        a.layout = super::Watch::at(super::location::path(&directory));
    }
    if a.layout.watching() {
        a.layout.poll(std::time::Instant::now());
        ctx.request_repaint_after(super::POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_and_pending_audio_defer_layout_but_empty_apps_do_not_wait() {
        let mut a = App::empty();
        assert!(ready(&a, false));
        a.startup.pending = true;
        assert!(!ready(&a, true));
        assert!(!ready(&a, false));
        a.startup.pending = false;
        assert!(ready(&a, true));
    }

    #[test]
    fn silent_audio_and_pending_plugins_do_not_poll_an_installed_watch() {
        use crate::play::{audition::Audition, playback};
        use cat_smf_editor::model::{Music, Track};
        let mut a = crate::view::tests::app(Music {
            format: 0,
            division: 480,
            tracks: vec![Track { events: Vec::new() }],
        });
        a.playback.pending = transport::prepare(&a, playback::Mode::AllTracks);
        assert!(a.playback.pending.is_some());
        a.audition = Audition::default();
        assert!(!ready(&a, false));
        let pending = a.playback.pending.take().unwrap();
        a.playback.active = Some(pending.active);
        assert!(
            !ready(&a, false),
            "an active silent sequence still defers layout"
        );
        assert!(ready(&a, true));
        a.playback.active = None;
        assert!(
            ready(&a, false),
            "stopping before any sound must not prevent layout"
        );
    }
}
