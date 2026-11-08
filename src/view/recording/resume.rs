//! When the keyboard opens and closes: by itself beside the Song window, and with the app.
use super::*;

/// At startup, before the first frame.
pub(crate) fn begin(a: &mut App) {
    a.patterns.recording.resuming = a.config.phrase_loop;
    a.patterns.recording.starting = true;
}

/// Opens as Shift+R in the Song window once it has a place on the screen, so the keyboard
/// avoids it: always while the song has patterns, and at startup when the app closed with
/// phrase loop on. Without patterns there is no Song window and the editor opens it.
pub(super) fn follow(a: &mut App, ctx: &egui::Context) {
    if a.patterns.recording.is_open() || a.loaded.is_none() {
        a.patterns.recording.resuming = false;
        a.patterns.recording.starting = false;
        return;
    }
    let song = super::super::patterns::mode::song_window(a);
    let resuming = a.patterns.recording.resuming;
    if !resuming && !(song && a.patterns.recording.failed != Some(a.song_id)) {
        a.patterns.recording.starting = false;
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
    // The main window first appears at the end of its first frame and takes the front then, so
    // a keyboard opened during that frame would lose the front to it.
    if ctx.cumulative_pass_nr_for(egui::ViewportId::ROOT) == 0 {
        return;
    }
    let pane = song.then_some(Pane::Song);
    // The keyboard is placed beside the window showing the Song, its own or the one holding
    // its pane, so it waits for that window's place on the screen.
    if let Some(p) = pane
        && input_for(ctx, locate::viewport_of(a, p.view()), |i| {
            i.viewport().outer_rect
        })
        .flatten()
        .is_none()
    {
        return;
    }
    a.patterns.recording.resuming = false;
    let starting = std::mem::take(&mut a.patterns.recording.starting);
    let back = starting && !a.config.keyboard_focused;
    // The grid the app closed with keeps the keys, since a pane that loses them closes it.
    let grid = back && focus::focused(a) == ViewKind::StepSequencer;
    let view = pane.map_or(locate::opener(a, ctx), Pane::view);
    if let Err(error) = open_from(a, ctx, view, pane, false, !grid) {
        a.patterns.recording.failed = Some(a.song_id);
        a.load_error = Some(error);
        return;
    }
    // The new keyboard window takes the focus; it goes back where the app closed with it.
    if back && let Some(s) = a.patterns.recording.session.as_mut() {
        s.focused = false;
        let origin = s.origin;
        if grid {
            super::hand_back(
                a,
                ctx,
                ViewKind::StepSequencer,
                locate::viewport_of(a, ViewKind::StepSequencer),
            );
        } else {
            super::hand_back(a, ctx, view, origin);
        }
    }
}

/// Brings the keyboard back from its minimized state, to the front.
pub(crate) fn restore(ctx: &egui::Context) {
    send_to(
        ctx,
        window::viewport(),
        egui::ViewportCommand::Minimized(false),
    );
    send_to(ctx, window::viewport(), egui::ViewportCommand::Focus);
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
