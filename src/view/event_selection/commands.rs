use super::*;
use crate::view::commands::{Command, Screen};
impl State {
    pub(crate) fn command_screen(&self) -> Screen {
        if self.menu {
            Screen::SelectionMenu
        } else {
            Screen::Selection
        }
    }
}
pub(crate) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if matches!(s, Screen::Selection | Screen::SelectionMenu) {
        out.push(Command::plain(
            s,
            "generate-same-measures",
            "Generate same measures",
            if s == Screen::SelectionMenu {
                &[egui::Key::G]
            } else {
                &[]
            },
            |a, c| {
                crate::view::palette::open_same_measure(a, c, true);
            },
        ));
    }
    if s == Screen::Main && count(a) > 0 {
        out.push(Command::plain(
            s,
            "selection-begin",
            "範囲選択",
            &[egui::Key::S],
            |a, _| super::begin(a),
        ));
    }
    if s == Screen::Selection {
        out.push(Command::plain(
            s,
            "selection-menu",
            "範囲menu",
            &[egui::Key::S],
            |a, _| a.event_selection.menu = true,
        ));
        out.push(Command::plain(
            s,
            "selection-cancel",
            "範囲取消",
            &[egui::Key::Escape],
            |a, _| a.event_selection = State::default(),
        ));
    }
    if s == Screen::SelectionMenu {
        out.push(Command::plain(
            s,
            "selection-close",
            "閉じる",
            &[egui::Key::Escape],
            |a, _| a.event_selection.menu = false,
        ));
        let mut choices = vec![
            (
                "selection-repeat",
                "A-B repeat",
                egui::Key::A,
                actions::Action::Repeat,
            ),
            (
                "selection-delete",
                "イベント削除",
                egui::Key::D,
                actions::Action::DeleteEvents,
            ),
        ];
        if whole(a) {
            choices.extend([
                (
                    "selection-init",
                    "Track初期化",
                    egui::Key::I,
                    actions::Action::Init,
                ),
                (
                    "selection-delete-track",
                    "Track削除",
                    egui::Key::T,
                    actions::Action::DeleteTrack,
                ),
            ]);
        }
        for (id, name, key, action) in choices {
            out.push(Command::plain(s, id, name, &[key], move |a, _| {
                actions::apply(a, action)
            }));
        }
    }
}
