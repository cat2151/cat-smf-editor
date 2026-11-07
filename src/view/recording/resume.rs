//! When the keyboard opens and closes: by itself beside the Song window, and with the app.
use super::*;

/// At startup, before the first frame.
pub(crate) fn begin(a: &mut App) {
    a.patterns.recording.resuming = a.config.phrase_loop;
}

/// Opens as Shift+R in the Song window once it has a place on the screen, so the keyboard
/// avoids it: always while the song has patterns, and at startup when the app closed with
/// phrase loop on. Without patterns there is no Song window and the editor opens it.
pub(super) fn follow(a: &mut App, ctx: &egui::Context) {
    if a.patterns.recording.is_open() || a.loaded.is_none() {
        a.patterns.recording.resuming = false;
        return;
    }
    let song = super::super::patterns::mode::song_window(a);
    let resuming = a.patterns.recording.resuming;
    if !resuming && !(song && a.patterns.recording.failed != Some(a.song_id)) {
        return;
    }
    // A confirmation, such as the exit one, would lose its keys to the new keyboard window.
    if a.missing_last_mid.is_some()
        || a.config.navigation.departure.is_some()
        || a.config.navigation.close_approved
        || modal::is_open(a)
        || a.patterns.confirming_creation()
    {
        return;
    }
    let pane = song.then_some(Pane::Song);
    // The Song window takes the focus once at startup; the keyboard opens after it has.
    if let Some(p) = pane
        && (ctx
            .input_for(p.viewport(), |i| i.viewport().outer_rect)
            .is_none()
            || resuming && !ctx.input_for(p.viewport(), |i| i.focused))
    {
        return;
    }
    a.patterns.recording.resuming = false;
    let origin = pane.map_or(ctx.viewport_id(), Pane::viewport);
    if let Err(error) = open_from(a, ctx, origin, pane, false) {
        a.patterns.recording.failed = Some(a.song_id);
        a.load_error = Some(error);
        return;
    }
    // The new keyboard window takes the focus; it goes back where the app closed with it.
    if resuming
        && !a.config.keyboard_focused
        && let Some(s) = a.patterns.recording.session.as_mut()
    {
        s.focused = false;
        ctx.send_viewport_cmd_to(origin, egui::ViewportCommand::Focus);
    }
}

/// Brings the keyboard back from its minimized state, to the front.
pub(crate) fn restore(ctx: &egui::Context) {
    ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd_to(window::viewport(), egui::ViewportCommand::Focus);
}

pub(super) fn shut(a: &mut App, state: &mut State) {
    state.follow = None;
    if let Some(s) = state.session.take() {
        a.audition.stop_live();
        // Settings and the loop length survive closing and restarts; the key is reread at every open.
        a.config
            .select_keyboard_settings((*s.settings(), s.measures));
        setup::keep_voicing(a, &s);
    }
}

/// Closes the keyboard window as its own close button does, so the exit confirmation gets input.
/// `false` keeps it open with the commit error.
pub(crate) fn close_for_exit(a: &mut App) -> bool {
    let mut state = std::mem::take(&mut a.patterns.recording);
    let done = state.session.as_mut().is_none_or(|s| {
        // Closing from the editor moves the focus there first, so the last of the two counts.
        a.config.select_keyboard_focused(s.focused);
        play::close(a, s, s.clock.elapsed())
    });
    if done {
        shut(a, &mut state);
    }
    a.patterns.recording = state;
    done
}
