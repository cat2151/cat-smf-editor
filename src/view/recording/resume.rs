//! The keyboard the app closed with, phrase loop on, opens again at startup above the Song window.
use super::*;

/// At startup, before the first frame.
pub(crate) fn begin(a: &mut App) {
    a.patterns.recording.resuming = a.config.phrase_loop;
}

/// Opens as Shift+R in the Song window once it has a place on the screen, so the keyboard
/// avoids it; without patterns there is no Song window and the editor opens it.
pub(super) fn follow(a: &mut App, ctx: &egui::Context) {
    if !a.patterns.recording.resuming || a.patterns.recording.is_open() {
        a.patterns.recording.resuming = false;
        return;
    }
    if a.loaded.is_none() {
        a.patterns.recording.resuming = false;
        return;
    }
    if a.missing_last_mid.is_some() || modal::is_open(a) || a.patterns.confirming_creation() {
        return;
    }
    let pane = super::super::patterns::mode::workspace_open(a).then_some(Pane::Song);
    if let Some(p) = pane
        && ctx
            .input_for(p.viewport(), |i| i.viewport().outer_rect)
            .is_none()
    {
        return;
    }
    a.patterns.recording.resuming = false;
    let origin = pane.map_or(ctx.viewport_id(), Pane::viewport);
    if let Err(error) = open_from(a, ctx, origin, pane) {
        a.load_error = Some(error);
    }
}
