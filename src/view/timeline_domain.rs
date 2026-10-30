//! Playback and cursor application belongs to domain adapters, outside gesture state.
use super::timeline_controls::{Click, Screen};

pub(crate) fn click(screen: Screen, app: &mut crate::app::App, track: usize, tick: u64) {
    match screen.policy() {
        Click::SeekPlaying => crate::play::transport::timeline::seek(app, track, tick),
        Click::Song => crate::play::arrangement::start_from(app, None, tick),
        Click::Scene => {
            if let Some(scene) = app.patterns.workspace.scene {
                crate::play::arrangement::start_from(app, Some(scene), tick);
            }
        }
        Click::Cursor => {
            if crate::play::transport::timeline::running(app) {
                crate::play::transport::timeline::seek(app, track, tick);
            } else {
                crate::cursor::moves::at_tick(app, tick);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_scene_never_falls_back_to_an_existing_song() {
        let (mut app, _, _, _, _) = crate::view::patterns::workspace_tests::verification_song();
        crate::play::transport::stop(&mut app);
        app.patterns.workspace.scene = None;
        click(Screen::Scene, &mut app, 0, 0);
        assert!(app.playback.pending.is_none() && app.playback.active.is_none());
    }
}
