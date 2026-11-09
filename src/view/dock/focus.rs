//! Which pane holds the keyboard. One pane of all windows has it.
use super::{focus_step, Host, Layout, ViewKind};
use crate::app::App;
use crate::view::commands::Screen;
use crate::view::patterns::workspace::{self, Pane};
use eframe::egui;

pub(crate) fn root(a: &App) -> Layout {
    a.layout
        .spec
        .resolve(a.note_mode, a.track_view.split.is_some())
}

/// Placed views that hold the focus exactly while they are open, so closing one hands the
/// keys back to the event list. The mixer counts as open while it holds the keys. The order is
/// the one the main window's keys follow.
pub(super) const OPENED: [ViewKind; 5] = [
    ViewKind::TrackOverview,
    ViewKind::Mixer,
    ViewKind::ParameterCurve,
    ViewKind::PianoRoll,
    ViewKind::TempoCurve,
];

pub(super) fn open(a: &App, view: ViewKind) -> bool {
    match view {
        ViewKind::PianoRoll => a.piano_roll.open,
        ViewKind::TempoCurve => a.tempo_curve.open,
        ViewKind::Mixer => a.playback.mixer_focused(),
        ViewKind::ParameterCurve => a.parameter_editor.open,
        ViewKind::TrackOverview => a.track_overview.open,
        ViewKind::EventList
        | ViewKind::Score
        | ViewKind::Analysis
        | ViewKind::Song
        | ViewKind::Scene
        | ViewKind::Keyboard
        | ViewKind::Piano
        | ViewKind::Help
        | ViewKind::StepSequencer => false,
    }
}

fn set_open(a: &mut App, view: ViewKind, open: bool) {
    match view {
        ViewKind::PianoRoll => a.piano_roll.open = open,
        ViewKind::TempoCurve => a.tempo_curve.open = open,
        ViewKind::Mixer => {
            a.playback.mixer_open = open;
            a.playback.mixer_focus |= open;
        }
        // Opening needs a curve to show; leaving it closes it as Esc does.
        ViewKind::ParameterCurve => {
            if !open && a.parameter_editor.open {
                a.parameter_editor.close();
            }
        }
        // Opening reads the cursor's measure; leaving it closes it as Esc does.
        ViewKind::TrackOverview => {
            if !open {
                a.track_overview.open = false;
            }
        }
        ViewKind::EventList
        | ViewKind::Score
        | ViewKind::Analysis
        | ViewKind::Song
        | ViewKind::Scene
        | ViewKind::Keyboard
        | ViewKind::Piano
        | ViewKind::Help
        | ViewKind::StepSequencer => {}
    }
}

/// The event list unless another placed pane took the focus.
pub(crate) fn focused(a: &App) -> ViewKind {
    let views = root(a).views();
    if let Some(view) = OPENED
        .into_iter()
        .find(|view| views.contains(view) && open(a, *view) && !folded(a, *view))
    {
        return view;
    }
    a.pane_focus
        .filter(|view| views.contains(view) && !OPENED.contains(view) && focusable(a, *view))
        .unwrap_or(ViewKind::EventList)
}

/// Help shows the keys of the view that last had the focus, and the piano plays by mouse beside
/// the keyboard, which keeps the keys; neither takes the focus.
pub(super) fn takes_focus(view: ViewKind) -> bool {
    !matches!(view, ViewKind::Help | ViewKind::Piano)
}

fn focusable(a: &App, view: ViewKind) -> bool {
    takes_focus(view) && shown(a, view) && !folded(a, view) && !hidden(a, view)
}

fn folded(a: &App, view: ViewKind) -> bool {
    super::collapse::folded(a, view)
}

fn hidden(a: &App, view: ViewKind) -> bool {
    super::tabs::hidden(a, view)
}

/// A placed view whose pane stays empty while it has nothing to show cannot hold the focus.
pub(super) fn shown(a: &App, view: ViewKind) -> bool {
    match view {
        ViewKind::Analysis => a.analysis.is_open(),
        ViewKind::Song => workspace::shown(a, Pane::Song),
        ViewKind::Scene => workspace::shown(a, Pane::Scene),
        ViewKind::Keyboard | ViewKind::Piano => a.patterns.recording.is_open(),
        ViewKind::StepSequencer => crate::view::drums::is_open(a),
        ViewKind::ParameterCurve => a.parameter_editor.open,
        ViewKind::TrackOverview => a.track_overview.open,
        ViewKind::EventList
        | ViewKind::Score
        | ViewKind::PianoRoll
        | ViewKind::TempoCurve
        | ViewKind::Mixer
        | ViewKind::Help => true,
    }
}

/// The focused pane owns the keys of `screen`, which it shares with its floating window.
pub(crate) fn owns(a: &App, screen: Screen) -> bool {
    self::screen(focused(a)) == screen
}

/// Gives `view` the focus, opening its folded pane; the placed views in `OPENED` open or close
/// with it.
pub(crate) fn set(a: &mut App, view: ViewKind) {
    super::collapse::unfold(a, view);
    a.pane_focus = Some(view);
    for kind in OPENED {
        if placed(a, kind) {
            set_open(a, kind, view == kind);
        }
    }
}

/// `view` sits in a pane rather than its own floating window.
pub(crate) fn placed(a: &App, view: ViewKind) -> bool {
    root(a).contains(view)
}

/// `view` sits in a dock window, which draws it and finishes its frame, so the main window
/// leaves it alone. A Views preview still shows it floating in the main window.
pub(crate) fn in_dock(a: &App, ctx: &egui::Context, view: ViewKind) -> bool {
    !crate::view::command_input::preview_active(ctx)
        && matches!(root(a).host_of(view), Some(Host::Dock(_)))
}

/// The window holding the focused pane.
fn focus_host(a: &App, layout: &Layout) -> Host {
    layout.host_of(focused(a)).unwrap_or(Host::Root)
}

/// The view owning the keys of dock window `index`: its focused pane, or its first pane that can
/// hold the focus, which `claim` hands the focus to once a key reaches the window. A window whose
/// panes are all empty still names its first pane, so what it opens shows in it.
pub(crate) fn dock_view(a: &App, index: usize) -> Option<ViewKind> {
    let layout = root(a);
    let views = layout.docks.get(index)?.node.views();
    let view = focused(a);
    if views.contains(&view) {
        return Some(view);
    }
    views
        .iter()
        .copied()
        .find(|view| focusable(a, *view))
        .or_else(|| {
            views
                .into_iter()
                .find(|view| takes_focus(*view) && !folded(a, *view) && !hidden(a, *view))
        })
}

/// A key typed into dock window `index` goes to its panes.
pub(crate) fn claim(a: &mut App, index: usize) {
    if let Some(view) = dock_view(a, index)
        && view != focused(a)
    {
        set(a, view);
    }
}

/// The screen owning the main window's keys while `view` has the focus.
pub(crate) fn screen(view: ViewKind) -> Screen {
    match view {
        ViewKind::EventList => Screen::Main,
        ViewKind::Score => Screen::Score,
        ViewKind::PianoRoll | ViewKind::TempoCurve | ViewKind::ParameterCurve => Screen::Timeline,
        ViewKind::Mixer => Screen::Mixer,
        ViewKind::Analysis => Screen::Analysis,
        ViewKind::Song => Screen::Song,
        ViewKind::Scene => Screen::Scene,
        ViewKind::Keyboard | ViewKind::Piano => Screen::Recording,
        ViewKind::Help => Screen::Help,
        ViewKind::StepSequencer => Screen::Drums,
        ViewKind::TrackOverview => Screen::Overview,
    }
}

/// Moves to the next pane of the focused pane's window, passing over empty ones.
pub(crate) fn step(a: &mut App, forward: bool) {
    let layout = root(a);
    let current = focused(a);
    let node = layout.node(focus_host(a, &layout));
    let mut next = Some(current);
    for _ in 0..node.views().len() {
        next = focus_step(node, next, forward);
        if let Some(view) = next.filter(|v| focusable(a, *v)) {
            if view != current {
                set(a, view);
            }
            return;
        }
    }
}

/// The tree of the focused pane's window, folded panes included.
pub(crate) fn window(a: &App) -> super::Node {
    let layout = root(a);
    layout.node(focus_host(a, &layout)).clone()
}

/// Panes in the focused pane's window that can take the focus, hidden tabs left out.
pub(crate) fn panes(a: &App) -> usize {
    let layout = root(a);
    layout
        .node(focus_host(a, &layout))
        .views()
        .into_iter()
        .filter(|view| takes_focus(*view) && !folded(a, *view) && !hidden(a, *view))
        .count()
}

/// Several panes share the windows, so the focused one gets a frame.
pub(crate) fn framed(a: &App) -> bool {
    root(a).views().len() > 1
}

/// A press inside the pane gives it the focus, unless a layer above it took the press.
pub(crate) fn click(a: &mut App, ui: &egui::Ui, rect: egui::Rect, view: ViewKind) {
    if takes_focus(view)
        && !folded(a, view)
        && !hidden(a, view)
        && ui.is_enabled()
        && ui.ctx().input(|i| i.pointer.any_pressed())
        && ui.rect_contains_pointer(rect)
    {
        set(a, view);
    }
}

#[cfg(test)]
mod tests;
