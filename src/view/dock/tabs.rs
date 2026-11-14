//! Views sharing one pane as tabs: which one each group shows. Nothing is remembered across runs,
//! so every group starts on its first tab.
use super::{focus, Layout, Node, ViewKind};
use crate::app::App;
use eframe::egui;

/// The tab each group shows, the group named by its first view.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Shown(Vec<Entry>);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    first: ViewKind,
    view: ViewKind,
    /// The focused view when the group was last drawn.
    seen: Option<ViewKind>,
}

impl Shown {
    fn entry(&mut self, first: ViewKind) -> &mut Entry {
        if let Some(i) = self.0.iter().position(|e| e.first == first) {
            return &mut self.0[i];
        }
        self.0.push(Entry {
            first,
            view: first,
            seen: None,
        });
        self.0.last_mut().unwrap()
    }

    fn stored(&self, views: &[ViewKind]) -> ViewKind {
        self.0
            .iter()
            .find(|e| Some(&e.first) == views.first())
            .map(|e| e.view)
            .filter(|view| views.contains(view))
            .unwrap_or(views[0])
    }
}

fn find(node: &Node, view: ViewKind) -> Option<&[ViewKind]> {
    match node {
        Node::View(_) => None,
        Node::Tabs(views) => views.contains(&view).then_some(views.as_slice()),
        Node::Row(children) | Node::Column(children) => {
            children.iter().find_map(|(child, _)| find(child, view))
        }
    }
}

/// The tabs `view` sits among, if it sits in a group.
pub(crate) fn group(layout: &Layout, view: ViewKind) -> Option<Vec<ViewKind>> {
    std::iter::once(&layout.root)
        .chain(layout.docks.iter().map(|dock| &dock.node))
        .find_map(|node| find(node, view))
        .map(<[ViewKind]>::to_vec)
}

/// The tab `views` shows now.
pub(crate) fn shown(a: &App, views: &[ViewKind]) -> ViewKind {
    fallback(a, views, a.pane_tabs.stored(views))
}

/// A tab that holds the focus exactly while it is open shows the first tab once it closed.
fn fallback(a: &App, views: &[ViewKind], view: ViewKind) -> ViewKind {
    if view != ViewKind::TrackOverview && focus::OPENED.contains(&view) && !focus::open(a, view) {
        views[0]
    } else {
        view
    }
}

/// `view` sits behind another tab of its group.
pub(crate) fn hidden(a: &App, view: ViewKind) -> bool {
    group(&focus::root(a), view).is_some_and(|views| shown(a, &views) != view)
}

/// The tab to draw this frame. A view of the group that took the focus since the last draw,
/// as one opened by its own key does, comes to the front.
pub(crate) fn sync(a: &mut App, views: &[ViewKind]) -> ViewKind {
    let mut view = shown(a, views);
    let focused = focus::focused(a);
    let entry = a.pane_tabs.entry(views[0]);
    if entry.seen != Some(focused) {
        if views.contains(&focused) {
            view = focused;
        }
        entry.seen = Some(focused);
    }
    entry.view = view;
    view
}

/// Shows `view` and gives it the focus; a view that takes no focus leaves the focus where it is.
pub(crate) fn select(a: &mut App, views: &[ViewKind], view: ViewKind) {
    a.pane_tabs.entry(views[0]).view = view;
    if !focus::takes_focus(view) {
        return;
    }
    focus::set(a, view);
}

/// A tab the keys can move to: one that holds the focus once shown.
fn reachable(a: &App, view: ViewKind) -> bool {
    focus::takes_focus(view)
        && (matches!(
            view,
            ViewKind::TrackOverview | ViewKind::PianoRoll | ViewKind::TempoCurve | ViewKind::Mixer
        ) || focus::shown(a, view))
}

/// Moves the focused pane's group to its next (or previous) tab, wrapping around and passing
/// over tabs that could not hold the focus.
pub(crate) fn step(a: &mut App, forward: bool) {
    let Some(views) = group(&focus::root(a), focus::focused(a)) else {
        return;
    };
    let n = views.len();
    let current = views
        .iter()
        .position(|v| *v == shown(a, &views))
        .unwrap_or(0);
    for k in 1..n {
        let i = if forward {
            (current + k) % n
        } else {
            (current + n - k) % n
        };
        if reachable(a, views[i]) {
            return select(a, &views, views[i]);
        }
    }
}

/// The focused pane sits among several tabs.
pub(crate) fn switchable(a: &App) -> bool {
    group(&focus::root(a), focus::focused(a)).is_some_and(|views| views.len() > 1)
}

/// The line of tab names; the shown one is highlighted. Returns the clicked tab.
pub(crate) fn header(ui: &mut egui::Ui, views: &[ViewKind], shown: ViewKind) -> Option<ViewKind> {
    ui.horizontal(|ui| {
        let mut clicked = None;
        for view in views {
            if ui.selectable_label(*view == shown, view.name()).clicked() {
                clicked = Some(*view);
            }
        }
        clicked
    })
    .inner
}

#[cfg(test)]
mod tests;
