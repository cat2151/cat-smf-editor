use super::*;
use crate::view::commands::{Command, Screen};
#[derive(Clone, Copy)]
pub(super) enum Action {
    Move(isize, isize),
    Activate,
    Pick,
    AddRows,
    Toggle,
    Menu,
    Query,
    Confirm,
    Cancel,
    Mute,
    Velocity(u8, u8),
    Swing(Option<u8>),
    Common(super::super::commands::Op),
}
pub(crate) fn collect(a: &App, screen: Screen, out: &mut Vec<Command>) {
    let own = if a.drum_editor.kit.has_input() {
        Screen::DrumInput
    } else {
        Screen::Drums
    };
    if own != screen {
        return;
    }
    if a.drum_editor.kit.picker.is_some() {
        super::picker::collect(a, screen, out);
        return;
    }
    if !a.drum_editor.kit.has_input() {
        out.push(Command::plain(screen, "effect.open", &[Key::X], |a, _| {
            crate::play::mixer::open_effects(a, false)
        }));
    }
    let mut add = |id, keys: &[Key], action| {
        let command = Command::plain(screen, id, keys, move |a, ctx| {
            if !a.drum_editor.kit.composing
                && !ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
            {
                apply(a, action);
            }
        });
        out.push(
            if matches!(
                action,
                Action::Move(..) | Action::Common(super::super::commands::Op::Page(..))
            ) {
                command.repeating()
            } else {
                command
            },
        );
    };
    add("drum.cancel", &[Key::Escape], Action::Cancel);
    if a.drum_editor.kit.has_input() {
        add("drum.confirm", &[Key::Enter], Action::Confirm);
        out.push(Command::new(
            screen,
            "app.stop",
            &[(Key::Space, egui::Modifiers::SHIFT)],
            |a, _| apply(a, Action::Common(super::super::commands::Op::Stop)),
        ));
        return;
    }
    for (id, keys, action) in [
        ("kit.note-picker", vec![Key::N], Action::Pick),
        ("kit.add-rows", vec![Key::A], Action::AddRows),
        (
            "kit.row-next",
            vec![Key::J, Key::ArrowDown],
            Action::Move(1, 0),
        ),
        (
            "kit.row-prev",
            vec![Key::K, Key::ArrowUp],
            Action::Move(-1, 0),
        ),
        (
            "kit.field-next",
            vec![Key::L, Key::ArrowRight],
            Action::Move(0, 1),
        ),
        (
            "kit.field-prev",
            vec![Key::H, Key::ArrowLeft],
            Action::Move(0, -1),
        ),
        ("kit.activate", vec![Key::Enter], Action::Activate),
        ("kit.toggle", vec![Key::F, Key::Space], Action::Toggle),
        ("kit.query", vec![Key::Slash], Action::Query),
        ("kit.mute", vec![Key::M], Action::Mute),
        (
            "drum.a-prev",
            vec![Key::OpenBracket],
            Action::Common(super::super::commands::Op::Boundary(false, -1)),
        ),
        (
            "drum.a-next",
            vec![Key::CloseBracket],
            Action::Common(super::super::commands::Op::Boundary(false, 1)),
        ),
        (
            "drum.page-next",
            vec![Key::PageDown],
            Action::Common(super::super::commands::Op::Page(true)),
        ),
        (
            "drum.page-prev",
            vec![Key::PageUp],
            Action::Common(super::super::commands::Op::Page(false)),
        ),
        (
            "drum.loop",
            vec![Key::R],
            Action::Common(super::super::commands::Op::Loop),
        ),
        (
            "drum.scope",
            vec![Key::S],
            Action::Common(super::super::commands::Op::Scope),
        ),
        (
            "app.help",
            vec![Key::F1],
            Action::Common(super::super::commands::Op::Help),
        ),
    ] {
        add(id, &keys, action);
    }
    for (id, key, modifiers, action) in [
        (
            "drum.b-prev",
            Key::OpenBracket,
            egui::Modifiers::SHIFT,
            Action::Common(super::super::commands::Op::Boundary(true, -1)),
        ),
        (
            "drum.b-next",
            Key::CloseBracket,
            egui::Modifiers::SHIFT,
            Action::Common(super::super::commands::Op::Boundary(true, 1)),
        ),
        (
            "kit.context",
            Key::F10,
            egui::Modifiers::SHIFT,
            Action::Menu,
        ),
        (
            "app.stop",
            Key::Space,
            egui::Modifiers::SHIFT,
            Action::Common(super::super::commands::Op::Stop),
        ),
    ] {
        out.push(Command::new(
            screen,
            id,
            &[(key, modifiers)],
            move |a, ctx| {
                if !a.drum_editor.kit.composing
                    && !ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
                {
                    apply(a, action);
                }
            },
        ));
    }
    if let Some(command) = super::super::commands::undo_command(a, screen, |a, ctx| {
        if !a.drum_editor.kit.composing
            && !ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_))))
        {
            apply(a, Action::Common(super::super::commands::Op::Undo));
        }
    }) {
        out.push(command);
    }
}
pub(super) fn apply(a: &mut App, action: Action) {
    let result = match action {
        Action::Move(y, x) => {
            let rows = rows(a);
            let k = &mut a.drum_editor.kit;
            let at = rows.iter().position(|n| n.slot == k.slot).unwrap_or(0);
            if let Some(row) = rows.get(
                at.saturating_add_signed(y)
                    .min(rows.len().saturating_sub(1)),
            ) {
                k.slot = row.slot;
                k.note = row.note;
            }
            k.reveal = true;
            let enabled = a.drum_editor.kit.note.is_some_and(|note| {
                a.loaded.as_ref().unwrap().patches[a.track]
                    .kit
                    .as_ref()
                    .unwrap()
                    .swing
                    .get(&note)
                    .is_some_and(|s| s.enabled)
            });
            let steps = playback::measure(a).map_or(16, |m| m.steps.len());
            let mut order = vec![0, steps + 3];
            if enabled {
                order.push(steps + 4);
            }
            order.extend(1..=steps + 2);
            let index = order
                .iter()
                .position(|f| *f == a.drum_editor.field)
                .unwrap_or(0);
            a.drum_editor.field = order[index.saturating_add_signed(x).min(order.len() - 1)];
            sync(a);
            Ok(())
        }
        Action::Activate => activate(a),
        Action::Mute => controls::mute(a),
        Action::Velocity(note, value) => controls::velocity(a, note, value),
        Action::Swing(value) => controls::swing(a, value),
        Action::Pick => super::picker::open(a),
        Action::AddRows => super::row_edit::add(a),
        Action::Toggle => {
            let steps = playback::measure(a).map_or(16, |m| m.steps.len());
            if !(2..steps + 2).contains(&a.drum_editor.field) {
                a.drum_editor.field = 2;
            }
            activate(a)
        }
        Action::Menu => {
            let m = playback::measure(a);
            match m {
                Ok(m) if (2..m.steps.len() + 2).contains(&a.drum_editor.field) => {
                    a.drum_editor.kit.menu = true;
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        Action::Query => {
            let k = &mut a.drum_editor.kit;
            k.query_before = Some((k.query.clone(), k.slot));
            k.focus = true;
            Ok(())
        }
        Action::Confirm => confirm(a),
        Action::Cancel => {
            cancel(a);
            Ok(())
        }
        Action::Common(op) => {
            if matches!(op, super::super::commands::Op::Stop) {
                a.audition.stop_key();
                a.drum_editor.kit.preview = None;
            }
            super::super::commands::apply(a, op);
            return;
        }
    };
    super::super::result(a, result);
}
