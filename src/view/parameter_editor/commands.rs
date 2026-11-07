use super::*;
use crate::view::commands::{Command, Screen};
use egui::Key;

pub(crate) fn collect(app: &App, screen: Screen, out: &mut Vec<Command>) {
    if screen == Screen::ParameterPicker {
        out.push(Command::plain(
            screen,
            "parameter.help",
            &[Key::F1],
            |a, _| crate::view::help::summon(a),
        ));
        out.push(Command::plain(
            screen,
            "parameter.cancel",
            &[Key::Escape],
            |a, _| {
                preview::stop(a);
                a.parameter_editor.picker = None;
                super::super::layers::open(a);
            },
        ));
        out.push(Command::plain(
            screen,
            "parameter.create",
            &[Key::Enter],
            |a, _| {
                if let Err(e) = create(a) {
                    a.parameter_editor.error = Some(e);
                }
            },
        ));
        for (key, modifiers, action) in crate::view::list_navigation::bindings() {
            out.push(
                Command::new(
                    screen,
                    match key {
                        Key::ArrowUp => "parameter.prev".into(),
                        Key::ArrowDown => "parameter.next".into(),
                        _ => format!("parameter.nav-{key:?}"),
                    },
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
        for (id, key, direction) in [
            ("parameter.value-up", Key::ArrowUp, 1.0),
            ("parameter.value-down", Key::ArrowDown, -1.0),
        ] {
            out.push(
                Command::plain(screen, id, &[key], move |a, _| {
                    curve::adjust_value(a, direction);
                })
                .repeating(),
            );
        }
        out.push(Command::plain(
            screen,
            "parameter.input-confirm",
            &[Key::Enter],
            |a, _| curve::commit_input(a),
        ));
        out.push(Command::plain(
            screen,
            "parameter.input-cancel",
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
    for (id, key, next, control) in [
        ("curve.next-point", Key::L, true, false),
        ("curve.prev-point", Key::H, false, false),
        ("curve.next-control", Key::J, true, true),
        ("curve.prev-control", Key::K, false, true),
    ] {
        out.push(Command::plain(screen, id, &[key], move |a, _| {
            curve::select(a, next, control)
        }));
    }
    out.push(Command::plain(screen, "curve.add", &[Key::A], |a, _| {
        curve::add(a)
    }));
    out.push(Command::plain(
        screen,
        "curve.edit",
        &[Key::Enter],
        |a, _| curve::begin_input(a),
    ));
    out.push(Command::plain(
        screen,
        "curve.remove",
        &[Key::Delete],
        |a, _| curve::remove(a),
    ));
    for (id, key, zoom_in) in [
        ("curve.zoom-in", Key::PageUp, true),
        ("curve.zoom-out", Key::PageDown, false),
    ] {
        out.push(Command::plain(screen, id, &[key], move |a, _| {
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
