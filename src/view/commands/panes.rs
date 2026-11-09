//! Moving the keyboard between the panes of a window and between tabs, and folding panes.
use super::*;
use crate::view::dock::{collapse, focus, tabs, Node};

pub(super) fn collect(a: &App, s: Screen, out: &mut Vec<Command>) {
    if s == Screen::Score {
        out.push(Command::plain(s, "pane.unfocus", &[Key::Escape], |a, _| {
            a.pane_focus = None
        }));
    }
    if !(matches!(s, Screen::Main | Screen::Score) || focus::owns(a, s)) {
        return;
    }
    folds(a, s, out);
    if tabs::switchable(a) {
        out.push(Command::new(
            s,
            "pane.tab-next",
            &[(Key::PageDown, Modifiers::CTRL)],
            |a, _| tabs::step(a, true),
        ));
        out.push(Command::new(
            s,
            "pane.tab-prev",
            &[(Key::PageUp, Modifiers::CTRL)],
            |a, _| tabs::step(a, false),
        ));
    }
    if focus::panes(a) < 2 {
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

/// The focused pane folds by key; every pane of its window, folded or one that takes no focus,
/// also folds or opens from the palette. A tab group is one entry, named by its shown tab.
fn folds(a: &App, s: Screen, out: &mut Vec<Command>) {
    let window = focus::window(a);
    if !collapse::foldable(&window) {
        return;
    }
    out.push(Command::new(
        s,
        "pane.fold",
        &[(Key::F6, Modifiers::CTRL)],
        |a, _| collapse::toggle(a, focus::focused(a)),
    ));
    for pane in collapse::panes(&window) {
        let view = pane.views()[0];
        let name = match pane {
            Node::Tabs(views) => tabs::shown(a, views),
            _ => view,
        };
        out.push(
            Command::new(s, format!("pane.fold-{view:?}"), &[], move |a, _| {
                collapse::toggle(a, view)
            })
            .named(format!("{} の折りたたみ", name.name()))
            .text_id("pane.fold-entry"),
        );
    }
}
