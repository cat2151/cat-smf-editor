//! The keyboard reopened at startup beside the Song window, with the focus where the app closed.
use super::*;
use crate::view::recording::{close_for_exit, resume, window};

/// Starts the app with the keyboard to resume and the Song window on the screen.
fn start(keyboard_focused: bool) -> (App, egui::Context) {
    let (mut a, c, _, _, _) = verification_song();
    a.config.phrase_loop = true;
    a.config.keyboard_focused = keyboard_focused;
    resume(&mut a);
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("window-geometry", Pane::Song.viewport())),
            [460.0_f32, 520.0, 760.0, 520.0],
        )
    });
    (a, c)
}
fn focus_sent(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data_mut(|d| d.remove_temp::<bool>(egui::Id::new(("workspace-focus", id))))
        .unwrap_or(false)
}

#[test]
fn the_keyboard_waits_until_the_song_window_has_taken_the_focus() {
    let (mut a, c) = start(true);
    // Created in the same frame, the keyboard would lose the focus to the Song window.
    focus(&c, egui::ViewportId::ROOT);
    for _ in 0..3 {
        frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    }
    assert!(!a.patterns.recording.is_open());
    focus(&c, Pane::Song.viewport());
    focus_sent(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(!focus_sent(&c, Pane::Song.viewport()));
}

#[test]
fn the_song_window_gets_the_focus_back_when_it_had_it_at_exit() {
    let (mut a, c) = start(false);
    focus(&c, egui::ViewportId::ROOT);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    focus(&c, Pane::Song.viewport());
    focus_sent(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(focus_sent(&c, Pane::Song.viewport()));
}

#[test]
fn the_app_closes_remembering_whether_the_keyboard_had_the_focus_last() {
    let (mut a, c) = start(false);
    focus(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    focus(&c, window::viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    // Clicking the editor's close button focuses the editor before the close.
    focus(&c, egui::ViewportId::ROOT);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(close_for_exit(&mut a));
    assert!(a.config.keyboard_focused);

    let (mut a, c) = start(true);
    focus(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    focus(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    focus(&c, egui::ViewportId::ROOT);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(close_for_exit(&mut a));
    assert!(!a.config.keyboard_focused);
}

#[test]
fn the_keyboard_stays_closed_while_the_exit_confirmation_waits() {
    let (mut a, c) = start(false);
    a.config.phrase_loop = false;
    resume(&mut a);
    focus(&c, Pane::Song.viewport());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    crate::play::mixer::volume::set(&mut a, 0, 0.5).unwrap();
    let mut close = egui::RawInput::default();
    close
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    let _ = c.run(close, |ctx| {
        crate::view::show(&mut a, ctx);
        a.close(ctx);
    });
    assert!(a.config.navigation.departure.is_some());
    // A reopened keyboard would take the focus and the keys that answer the confirmation.
    for _ in 0..3 {
        frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    }
    assert!(!a.patterns.recording.is_open());
}
