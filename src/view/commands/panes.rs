//! Moving the keyboard between the panes of the main window.
use super::*;
use crate::view::dock::focus;

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if s == Screen::Score {
        out.push(Command::plain(s, "pane.unfocus", &[Key::Escape], |a, _| {
            a.pane_focus = None
        }));
    }
    if !(matches!(s, Screen::Main | Screen::Score) || focus::owns(a, s)) || focus::panes(a) < 2 {
        return;
    }
    out.push(Command::plain(s, "pane.focus-next", &[Key::F6], |a, _| {
        focus::step(a, true)
    }));
    out.push(Command::new(
        s,
        "pane.focus-prev",
        &[(Key::F6, Modifiers::SHIFT)],
        |a, _| focus::step(a, false),
    ));
}
