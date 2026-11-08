//! Which view a window is showing, so that ownership is decided by view rather than by window.
use super::{focus, window, Host, ViewKind};
use crate::app::App;
use crate::view::patterns::workspace::Pane;
use eframe::egui;

/// The view `ctx` shows: a dock window's focused pane, the main window's own focused pane
/// (the event list while the focus is in a dock window), or the view of a window of its own.
/// `None` for a window no view lives in.
pub(crate) fn view_at(a: &App, ctx: &egui::Context) -> Option<ViewKind> {
    let id = ctx.viewport_id();
    match host(a, id) {
        Some(Host::Root) => Some(root_view(a)),
        Some(Host::Dock(index)) => focus::dock_view(a, index),
        None if id == crate::view::help::viewport() => Some(ViewKind::Help),
        None if id == a.analysis.viewport() => Some(ViewKind::Analysis),
        None if crate::view::recording::owns(id) => Some(ViewKind::Keyboard),
        None => [Pane::Scene, Pane::Song]
            .into_iter()
            .find(|p| p.viewport() == id)
            .map(Pane::view),
    }
}

/// The view a window opened from `ctx` belongs to; the main window's for a window without one.
pub(crate) fn opener(a: &App, ctx: &egui::Context) -> ViewKind {
    view_at(a, ctx).unwrap_or(ViewKind::EventList)
}

/// The pane window `ctx` is, if it splits into panes.
pub(crate) fn host_at(a: &App, ctx: &egui::Context) -> Option<Host> {
    host(a, ctx.viewport_id())
}

/// `ctx` is the window `view` is drawn in.
pub(crate) fn shows(a: &App, ctx: &egui::Context, view: ViewKind) -> bool {
    viewport_of(a, view) == ctx.viewport_id()
}

/// The window `view` is drawn in: the window holding its pane, or else its own window. Views
/// without a window of their own float in the main window.
pub(crate) fn viewport_of(a: &App, view: ViewKind) -> egui::ViewportId {
    match focus::root(a).host_of(view) {
        Some(Host::Root) => egui::ViewportId::ROOT,
        Some(Host::Dock(index)) => window::viewport(index),
        None => match view {
            ViewKind::Help => crate::view::help::viewport(),
            ViewKind::Analysis => a.analysis.viewport(),
            ViewKind::Keyboard => crate::view::recording::window::viewport(),
            ViewKind::Piano => crate::view::recording::piano_viewport(),
            ViewKind::Song => Pane::Song.viewport(),
            ViewKind::Scene => Pane::Scene.viewport(),
            ViewKind::EventList
            | ViewKind::Score
            | ViewKind::PianoRoll
            | ViewKind::TempoCurve
            | ViewKind::Mixer
            | ViewKind::StepSequencer
            | ViewKind::ParameterCurve
            | ViewKind::TrackOverview => egui::ViewportId::ROOT,
        },
    }
}

fn host(a: &App, id: egui::ViewportId) -> Option<Host> {
    if id == egui::ViewportId::ROOT {
        return Some(Host::Root);
    }
    window::index(a, id).map(Host::Dock)
}

fn root_view(a: &App) -> ViewKind {
    let view = focus::focused(a);
    if matches!(focus::root(a).host_of(view), Some(Host::Dock(_))) {
        ViewKind::EventList
    } else {
        view
    }
}

#[cfg(test)]
mod tests;
