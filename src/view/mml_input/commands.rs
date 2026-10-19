use super::*;
use crate::view::commands::{Command, Screen};
pub(crate) fn collect(_a: &App, s: Screen, out: &mut Vec<Command>) {
    if s != Screen::Mml {
        return;
    }
    out.push(Command::plain(
        s,
        "mml-confirm",
        "確定",
        &[egui::Key::Enter],
        |a, _| a.mml_input.command = Some(Action::Confirm),
    ));
    out.push(Command::plain(
        s,
        "mml-cancel",
        "取消",
        &[egui::Key::Escape],
        |a, _| a.mml_input.command = Some(Action::Cancel),
    ));
}
