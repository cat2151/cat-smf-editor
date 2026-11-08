//! The same score layout for the main view and Views previews.
use super::dock::{self, Axis, Node, ViewKind};
use super::patterns::workspace::Pane;
use super::{staff, ACTIVE_PANE};
use crate::app::App;
use eframe::egui;

type MapClick = Option<(usize, usize, u64)>;

pub(super) fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    browse_patch: &mut bool,
    replace_effect: &mut Option<usize>,
) -> MapClick {
    let layout = dock::focus::root(app);
    if !score_visible(app) || !layout.contains(ViewKind::Score) {
        staff::suspend(app);
    }
    let mut map_click = None;
    node(
        app,
        ui,
        &layout.root,
        &mut Leaf {
            browse_patch,
            replace_effect,
            map_click: &mut map_click,
            framed: dock::focus::framed(app),
        },
    );
    map_click
}

/// The panes of a dock window, which never holds the event list.
pub(crate) fn dock(app: &mut App, ui: &mut egui::Ui, tree: &Node) {
    let (mut browse_patch, mut replace_effect, mut map_click) = (false, None, None);
    node(
        app,
        ui,
        tree,
        &mut Leaf {
            browse_patch: &mut browse_patch,
            replace_effect: &mut replace_effect,
            map_click: &mut map_click,
            framed: dock::focus::framed(app),
        },
    );
}

/// The floating tick overlays show the same time axis and own the wheel while open.
fn score_visible(app: &App) -> bool {
    let floating = |open: bool, view| open && !dock::focus::placed(app, view);
    app.note_mode
        && app.track_view.split.is_none()
        && !floating(app.piano_roll.open, ViewKind::PianoRoll)
        && !floating(app.tempo_curve.open, ViewKind::TempoCurve)
}

struct Leaf<'a> {
    browse_patch: &'a mut bool,
    replace_effect: &'a mut Option<usize>,
    map_click: &'a mut MapClick,
    /// Several panes share the window, so the focused one gets a frame.
    framed: bool,
}

fn node(app: &mut App, ui: &mut egui::Ui, node: &Node, leaf: &mut Leaf) {
    let (axis, children) = match node {
        Node::View(view) => return self::view(app, ui, *view, leaf),
        Node::Row(children) => (Axis::Row, children),
        Node::Column(children) => (Axis::Column, children),
    };
    let rect = ui.available_rect_before_wrap();
    let spacing = ui.spacing().item_spacing;
    let gap = if axis == Axis::Row {
        spacing.x
    } else {
        spacing.y
    };
    let weights: Vec<f32> = children.iter().map(|(_, size)| *size).collect();
    // Laid out like `ui.columns`, but each pane has its own id: sibling panes sharing one
    // would share the state of their scroll areas.
    let mut panes: Vec<egui::Ui> = dock::split(rect, axis, &weights, gap)
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let mut pane = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("pane", i))
                    .max_rect(r)
                    .layout(egui::Layout::top_down_justified(egui::Align::LEFT)),
            );
            pane.set_width(r.width());
            pane
        })
        .collect();
    for ((child, _), pane) in children.iter().zip(&mut panes) {
        self::node(app, pane, child, leaf);
    }
    ui.advance_cursor_after_rect(rect);
}

fn view(app: &mut App, ui: &mut egui::Ui, view: ViewKind, leaf: &mut Leaf) {
    let rect = ui.max_rect();
    // The window stayed enabled for the pane holding the open input; the others still wait.
    if ui.is_enabled()
        && !super::presentation::background_enabled(app)
        && super::presentation::input_view(app, ui.ctx()) != Some(view)
    {
        ui.disable();
    }
    contents(app, ui, view, leaf);
    if leaf.framed {
        dock::focus::click(app, ui, rect, view);
        if dock::focus::focused(app) == view {
            ui.painter_at(rect).rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(2.0_f32, ACTIVE_PANE),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn contents(app: &mut App, ui: &mut egui::Ui, view: ViewKind, leaf: &mut Leaf) {
    match view {
        ViewKind::EventList => event_list(app, ui, leaf),
        ViewKind::Score => {
            if score_visible(app) {
                staff::show(app, ui);
            }
        }
        ViewKind::PianoRoll => super::piano_roll::pane(app, ui),
        ViewKind::TempoCurve => super::tempo_curve::pane(app, ui),
        ViewKind::Mixer => crate::play::mixer::pane(app, ui),
        ViewKind::Analysis => super::analysis::pane(app, ui),
        ViewKind::Song => super::patterns::workspace::leaf(app, ui, Pane::Song),
        ViewKind::Scene => super::patterns::workspace::leaf(app, ui, Pane::Scene),
        ViewKind::Keyboard => super::recording::pane(app, ui),
        ViewKind::Piano => super::recording::piano_pane(app, ui),
        ViewKind::Help => super::help::pane(app, ui),
        ViewKind::StepSequencer => super::drums::pane(app, ui),
        ViewKind::ParameterCurve => super::parameter_editor::pane(app, ui),
        ViewKind::TrackOverview => super::track_overview::pane(app, ui),
    }
}

fn event_list(app: &mut App, ui: &mut egui::Ui, leaf: &mut Leaf) {
    // Raw rows are too wide for two columns.
    let Some(tracks) = app.track_view.split.filter(|_| app.note_mode) else {
        if let Some(tick) = super::pane(
            app,
            ui,
            app.track,
            true,
            leaf.browse_patch,
            leaf.replace_effect,
        ) {
            *leaf.map_click = Some((app.track_view.active, app.track, tick));
        }
        return;
    };
    ui.columns(2, |panes| {
        let active = app.track_view.active;
        for slot in [active, 1 - active] {
            panes[slot].push_id(slot, |ui| {
                if let Some(tick) = super::pane(
                    app,
                    ui,
                    tracks[slot],
                    slot == active,
                    leaf.browse_patch,
                    leaf.replace_effect,
                ) {
                    *leaf.map_click = Some((slot, tracks[slot], tick));
                }
            });
            if slot == active {
                let rect = panes[slot].min_rect().expand(2.0);
                panes[slot].painter().with_clip_rect(rect).rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(2.0_f32, ACTIVE_PANE),
                    egui::StrokeKind::Inside,
                );
            }
        }
    });
}

#[cfg(test)]
mod tests;
