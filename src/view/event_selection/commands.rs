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
        let mut command = Command::operation(
            s,
            "event.generate-same-measures",
            crate::view::commands::Operation::SameMeasure {
                batch: true,
                number: None,
            },
        );
        if s == Screen::SelectionMenu {
            command.keys.push((egui::Key::G, egui::Modifiers::NONE));
        }
        out.push(command);
    }
    if s == Screen::Main && count(a) > 0 {
        out.push(Command::plain(
            s,
            "selection.begin",
            &[egui::Key::S],
            |a, _| super::begin(a),
        ));
    }
    if s == Screen::Selection {
        out.push(Command::plain(
            s,
            "selection.menu",
            &[egui::Key::S],
            |a, _| a.event_selection.menu = true,
        ));
        out.push(Command::plain(
            s,
            "selection.cancel",
            &[egui::Key::Escape],
            |a, _| a.event_selection = State::default(),
        ));
    }
    if s == Screen::SelectionMenu {
        out.push(Command::plain(
            s,
            "selection.close",
            &[egui::Key::Escape],
            |a, _| a.event_selection.menu = false,
        ));
        let mut choices = vec![
            ("selection.repeat", egui::Key::A, actions::Action::Repeat),
            (
                "selection.delete",
                egui::Key::D,
                actions::Action::DeleteEvents,
            ),
            (
                "selection.quantize",
                egui::Key::Q,
                actions::Action::Quantize,
            ),
        ];
        if crate::view::patterns::mode::editing(a, a.track).is_some() {
            choices.push(("selection.crop", egui::Key::C, actions::Action::Crop));
        }
        if whole(a) {
            choices.extend([
                ("selection.init", egui::Key::I, actions::Action::Init),
                (
                    "selection.delete-track",
                    egui::Key::T,
                    actions::Action::DeleteTrack,
                ),
            ]);
        }
        for (id, key, action) in choices {
            out.push(Command::plain(s, id, &[key], move |a, _| {
                actions::apply(a, action)
            }));
        }
    }
}
