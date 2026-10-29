use super::*;
use crate::view::commands::{Command, Screen};
use egui::Key;

pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen == Screen::ParameterPicker {
        out.push(Command::plain(
            screen,
            "parameter-help",
            "ヘルプ",
            &[Key::F1],
            |a, _| a.help_open = !a.help_open,
        ));
        out.push(Command::plain(
            screen,
            "parameter-cancel",
            "閉じる",
            &[Key::Escape],
            |a, _| {
                preview::stop(a);
                a.parameter_editor.picker = None;
                super::super::layers::open(a);
            },
        ));
        out.push(Command::plain(
            screen,
            "parameter-create",
            "Create automation track",
            &[Key::Enter],
            |a, _| {
                if let Err(e) = create(a) {
                    a.parameter_editor.error = Some(e);
                }
            },
        ));
        for (key, modifiers, action, name) in crate::view::list_navigation::bindings() {
            out.push(
                Command::new(
                    screen,
                    match key {
                        Key::ArrowUp => "parameter-prev".into(),
                        Key::ArrowDown => "parameter-next".into(),
                        _ => format!("parameter-nav-{key:?}"),
                    },
                    name,
                    &[(key, modifiers)],
                    move |a, _| {
                        if let Some(p) = &mut a.parameter_editor.picker {
                            let count = filtered(p).len();
                            p.reveal = true;
                            p.selected = action.apply(p.selected, count);
                        }
                        preview::start(a);
                    },
                )
                .repeating(),
            );
        }
    }
    if screen == Screen::ParameterInput {
        for (id, name, key, direction) in [
            ("parameter-value-up", "値を上げる", Key::ArrowUp, 1.0),
            ("parameter-value-down", "値を下げる", Key::ArrowDown, -1.0),
        ] {
            out.push(
                Command::plain(screen, id, name, &[key], move |a, _| {
                    curve::adjust_value(a, direction);
                })
                .repeating(),
            );
        }
        out.push(Command::plain(
            screen,
            "parameter-input-confirm",
            "数値確定",
            &[Key::Enter],
            |a, _| curve::commit_input(a),
        ));
        out.push(Command::plain(
            screen,
            "parameter-input-cancel",
            "数値取消",
            &[Key::Escape],
            |a, _| {
                a.parameter_editor.input = None;
                a.parameter_editor.draft = None;
            },
        ));
    }
    if screen != Screen::Timeline || !app.parameter_editor.open {
        return;
    }
    for (id, name, key, next, control) in [
        ("curve-next-point", "次のpoint", Key::L, true, false),
        ("curve-prev-point", "前のpoint", Key::H, false, false),
        ("curve-next-control", "次のcontrol", Key::J, true, true),
        ("curve-prev-control", "前のcontrol", Key::K, false, true),
    ] {
        out.push(Command::plain(screen, id, name, &[key], move |a, _| {
            curve::select(a, next, control)
        }));
    }
    out.push(Command::plain(
        screen,
        "curve-add",
        "Point追加",
        &[Key::A],
        |a, _| curve::add(a),
    ));
    out.push(Command::plain(
        screen,
        "curve-edit",
        "Point / Control編集",
        &[Key::Enter],
        |a, _| curve::begin_input(a),
    ));
    out.push(Command::plain(
        screen,
        "curve-remove",
        "Point削除",
        &[Key::Delete],
        |a, _| curve::remove(a),
    ));
    for (id, name, key, zoom_in) in [
        ("curve-zoom-in", "時間拡大", Key::PageUp, true),
        ("curve-zoom-out", "時間縮小", Key::PageDown, false),
    ] {
        out.push(Command::plain(screen, id, name, &[key], move |a, _| {
            let view = &mut a.parameter_editor.viewport;
            let z = view.time_zoom();
            let next = if zoom_in {
                z * super::super::timeline_zoom::CLICK
            } else {
                z / super::super::timeline_zoom::CLICK
            };
            view.zoom_time(next, view.width / 2.0);
        }));
    }
}
