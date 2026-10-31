use super::*;
use crate::view::commands::{Command, Screen};
use egui::Modifiers;

#[derive(Clone, Copy)]
pub(super) enum Op {
    Move(i32, i32),
    Page(bool),
    Activate,
    Confirm,
    Cancel,
    Close,
    Help,
    Toggle,
    Scope,
    Loop,
    Boundary(bool, i32),
    Stop,
    Undo,
    Create,
}
pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    let own = if app.drum_editor.confirm_create {
        Screen::DrumCreate
    } else if app.drum_editor.input.is_some() {
        Screen::DrumInput
    } else {
        Screen::Drums
    };
    if screen != own {
        return;
    }
    let mut add = |id, name, keys: &[Key], op| {
        let command = Command::plain(screen, id, name, keys, move |a, _| apply(a, op));
        out.push(if matches!(op, Op::Move(..) | Op::Page(..)) {
            command.repeating()
        } else {
            command
        });
    };
    add("drum-cancel", "閉じる / 取消", &[Key::Escape], Op::Cancel);
    if app.drum_editor.confirm_create {
        add("drum-create", "Yes", &[Key::Enter], Op::Create);
    } else if app.drum_editor.input.is_some() {
        add("drum-confirm", "確定", &[Key::Enter], Op::Confirm);
    } else {
        add("help", "ヘルプ", &[Key::F1], Op::Help);
        {
            add(
                "drum-row-next",
                "次の行",
                &[Key::J, Key::ArrowDown],
                Op::Move(1, 0),
            );
            add(
                "drum-row-prev",
                "前の行",
                &[Key::K, Key::ArrowUp],
                Op::Move(-1, 0),
            );
            add(
                "drum-step-next",
                "次の項目",
                &[Key::L, Key::ArrowRight],
                Op::Move(0, 1),
            );
            add(
                "drum-step-prev",
                "前の項目",
                &[Key::H, Key::ArrowLeft],
                Op::Move(0, -1),
            );
            add(
                "drum-page-next",
                "次の小節",
                &[Key::PageDown],
                Op::Page(true),
            );
            add(
                "drum-page-prev",
                "前の小節",
                &[Key::PageUp],
                Op::Page(false),
            );
            add(
                "drum-activate",
                "音色 / ノート番号 / Step",
                &[Key::Enter],
                Op::Activate,
            );
            add("drum-toggle", "Step", &[Key::F, Key::Space], Op::Toggle);
            add("drum-scope", "Solo / All", &[Key::S], Op::Scope);
            add("drum-loop", "Loop", &[Key::R], Op::Loop);
            add(
                "drum-a-prev",
                "A −",
                &[Key::OpenBracket],
                Op::Boundary(false, -1),
            );
            add(
                "drum-a-next",
                "A +",
                &[Key::CloseBracket],
                Op::Boundary(false, 1),
            );
        }
    }
    if screen == Screen::Drums {
        for (id, key, delta) in [
            ("drum-b-prev", Key::OpenBracket, -1),
            ("drum-b-next", Key::CloseBracket, 1),
        ] {
            out.push(Command::new(
                screen,
                id,
                "B",
                &[(key, Modifiers::SHIFT)],
                move |a, _| apply(a, Op::Boundary(true, delta)),
            ));
        }
    }
    out.push(Command::new(
        screen,
        "stop",
        "停止",
        &[(Key::Space, Modifiers::SHIFT)],
        |a, _| apply(a, Op::Stop),
    ));
    if !app.drum_editor.confirm_create && app.drum_editor.input.is_none() && app.undo.available() {
        out.push(Command::new(
            screen,
            "undo",
            "元に戻す",
            &[(Key::Z, Modifiers::CTRL)],
            |a, _| apply(a, Op::Undo),
        ));
    }
}
pub(super) fn apply(app: &mut App, op: Op) {
    if !is_open(app) {
        return;
    }
    let s = &mut app.drum_editor;
    let r = match op {
        Op::Cancel => {
            if s.input.is_some() {
                s.input = None;
            } else {
                dismiss(app);
            }
            Ok(())
        }
        Op::Close => {
            dismiss(app);
            Ok(())
        }
        Op::Help => {
            crate::view::help::summon(app);
            Ok(())
        }
        Op::Move(y, x) => {
            s.row = s.row.saturating_add_signed(-(y as isize)).min(3);
            let last = playback::measure(app).map_or(17, |m| m.steps.len() + 1);
            app.drum_editor.field = app
                .drum_editor
                .field
                .saturating_add_signed(x as isize)
                .min(last);
            Ok(())
        }
        Op::Page(next) => playback::page(app, next),
        Op::Activate => activate(app),
        Op::Confirm => confirm(app),
        Op::Toggle => {
            app.drum_editor.field = app.drum_editor.field.max(2);
            activate(app)
        }
        Op::Scope => {
            app.drum_editor.all_tracks = !app.drum_editor.all_tracks;
            playback::restart(app)
        }
        Op::Loop => {
            app.drum_editor.loop_enabled = !app.drum_editor.loop_enabled;
            if app.drum_editor.loop_enabled {
                playback::restart(app)
            } else {
                playback::close(app);
                Ok(())
            }
        }
        Op::Boundary(end, delta) => playback::boundary(app, end, delta),
        Op::Stop => {
            app.drum_editor.loop_enabled = false;
            crate::play::transport::stop(app);
            Ok(())
        }
        Op::Undo => app.undo_music(),
        Op::Create => create_track(app),
    };
    result(app, r);
}
