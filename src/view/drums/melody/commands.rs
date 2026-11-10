use super::super::commands::Op;
use super::*;
use crate::view::commands::{Command, Screen};
use egui::Modifiers;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Layers,
    Move(isize, isize),
    Octave(bool),
    Toggle,
    Resize(Length),
    /// `None` steps to the next set.
    Rows(Option<harmony::Set>),
    /// A row's note name was clicked: sound its pitch without editing.
    Listen(u8),
    Common(Op),
}

pub(crate) fn collect(a: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen != Screen::Drums {
        return;
    }
    let mut add = |id, keys: &[(Key, Modifiers)], action| {
        let command = Command::new(screen, id, keys, move |a, _| apply(a, action));
        out.push(
            if matches!(action, Action::Move(..) | Action::Common(Op::Page(..))) {
                command.repeating()
            } else {
                command
            },
        );
    };
    let none = Modifiers::NONE;
    let shift = Modifiers::SHIFT;
    add(
        "drum.cancel",
        &[(Key::Escape, none)],
        Action::Common(Op::Cancel),
    );
    add("app.help", &[(Key::F1, none)], Action::Common(Op::Help));
    add("layer.open", &[(Key::T, shift)], Action::Layers);
    add(
        "effect.open",
        &[(Key::X, none)],
        Action::Common(Op::Effects),
    );
    add(
        "melody.row-next",
        &[(Key::J, none), (Key::ArrowDown, none)],
        Action::Move(1, 0),
    );
    add(
        "melody.row-prev",
        &[(Key::K, none), (Key::ArrowUp, none)],
        Action::Move(-1, 0),
    );
    add(
        "melody.step-next",
        &[(Key::L, none), (Key::ArrowRight, none)],
        Action::Move(0, 1),
    );
    add(
        "melody.step-prev",
        &[(Key::H, none), (Key::ArrowLeft, none)],
        Action::Move(0, -1),
    );
    add(
        "melody.toggle",
        &[(Key::F, none), (Key::Space, none)],
        Action::Toggle,
    );
    add(
        "melody.extend",
        &[(Key::L, shift)],
        Action::Resize(Length::Longer),
    );
    add(
        "melody.shorten",
        &[(Key::H, shift)],
        Action::Resize(Length::Shorter),
    );
    add(
        "melody.octave-down",
        &[(Key::J, shift)],
        Action::Octave(false),
    );
    add("melody.octave-up", &[(Key::K, shift)], Action::Octave(true));
    add("melody.filter", &[(Key::Slash, none)], Action::Rows(None));
    add(
        "drum.page-next",
        &[(Key::PageDown, none)],
        Action::Common(Op::Page(true)),
    );
    add(
        "drum.page-prev",
        &[(Key::PageUp, none)],
        Action::Common(Op::Page(false)),
    );
    add("drum.scope", &[(Key::S, none)], Action::Common(Op::Scope));
    add("drum.loop", &[(Key::R, none)], Action::Common(Op::Loop));
    add(
        "drum.a-prev",
        &[(Key::OpenBracket, none)],
        Action::Common(Op::Boundary(false, -1)),
    );
    add(
        "drum.a-next",
        &[(Key::CloseBracket, none)],
        Action::Common(Op::Boundary(false, 1)),
    );
    add(
        "drum.b-prev",
        &[(Key::OpenBracket, shift)],
        Action::Common(Op::Boundary(true, -1)),
    );
    add(
        "drum.b-next",
        &[(Key::CloseBracket, shift)],
        Action::Common(Op::Boundary(true, 1)),
    );
    add("app.stop", &[(Key::Space, shift)], Action::Common(Op::Stop));
    if let Some(command) =
        super::super::commands::undo_command(a, screen, |a, _| apply(a, Action::Common(Op::Undo)))
    {
        out.push(command);
    }
}

pub(super) fn apply(a: &mut App, action: Action) {
    if a.drum_editor.melody.gesture.take().is_some() && matches!(action, Action::Common(Op::Cancel))
    {
        return;
    }
    let result = match action {
        Action::Layers => {
            crate::view::layers::open_from_sequencer(a);
            Ok(())
        }
        Action::Move(rows, steps) => {
            move_cursor(a, rows, steps);
            Ok(())
        }
        Action::Octave(up) => {
            octave(a, up);
            Ok(())
        }
        Action::Toggle => toggle(a),
        Action::Resize(length) => resize(a, length),
        Action::Rows(set) => {
            next_rows(a, set);
            Ok(())
        }
        Action::Listen(pitch) => preview(a, pitch),
        Action::Common(op) => {
            if matches!(op, Op::Stop | Op::Cancel) {
                a.audition.stop_key();
                a.drum_editor.kit.preview = None;
            }
            super::super::commands::apply(a, op);
            return;
        }
    };
    super::super::result(a, result);
}
