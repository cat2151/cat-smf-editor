use super::*;
use crate::view::commands::{Command, Screen};
pub(crate) fn collect(_: &App, s: Screen, out: &mut Vec<Command>) {
    if s != Screen::Mml {
        return;
    }
    out.push(Command::plain(
        s,
        "mml.confirm",
        &[egui::Key::Enter],
        |a, _| a.mml_input.command = Some(Action::Confirm),
    ));
    out.push(Command::plain(
        s,
        "mml.cancel",
        &[egui::Key::Escape],
        |a, _| a.mml_input.command = Some(Action::Cancel),
    ));
}
