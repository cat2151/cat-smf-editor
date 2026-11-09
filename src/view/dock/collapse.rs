//! Panes folded to one line, or one column inside a row. Nothing is remembered across runs.
use super::{focus, Axis, Layout, Node, ViewKind};
use crate::app::App;
use eframe::egui;

/// Folded panes, each named by its first view; a view sits once in a layout, so the name is
/// unique. Names whose view left the layout simply match nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Collapsed(Vec<ViewKind>);

impl Collapsed {
    pub(crate) fn contains(&self, pane: &Node) -> bool {
        first(pane).is_some_and(|view| self.0.contains(&view))
    }

    pub(crate) fn toggle(&mut self, pane: &Node) {
        let Some(view) = first(pane) else { return };
        if let Some(i) = self.0.iter().position(|v| *v == view) {
            self.0.remove(i);
        } else {
            self.0.push(view);
        }
    }
}

fn first(pane: &Node) -> Option<ViewKind> {
    pane.views().first().copied()
}

/// The panes of a window holding several of them get a toggle; a lone pane never folds. A tab
/// group is one pane.
pub(crate) fn foldable(node: &Node) -> bool {
    panes(node).len() > 1
}

/// The panes of `node` in focus order: its views and tab groups.
pub(crate) fn panes(node: &Node) -> Vec<&Node> {
    match node {
        Node::View(_) | Node::Tabs(_) => vec![node],
        Node::Row(children) | Node::Column(children) => children
            .iter()
            .flat_map(|(child, _)| panes(child))
            .collect(),
    }
}

/// The pane `view` sits in: its tab group, or the view alone.
fn pane(layout: &Layout, view: ViewKind) -> Node {
    super::tabs::group(layout, view).map_or(Node::View(view), Node::Tabs)
}

fn shares(layout: &Layout, view: ViewKind) -> bool {
    layout
        .host_of(view)
        .is_some_and(|host| foldable(layout.node(host)))
}

/// `view`'s pane is folded; every tab of a group folds with it.
pub(crate) fn folded(a: &App, view: ViewKind) -> bool {
    let layout = focus::root(a);
    a.pane_collapse.contains(&pane(&layout, view)) && shares(&layout, view)
}

/// Folds or opens `view`'s pane, a whole tab group at once. Folding the focused pane hands the
/// focus on as F6 does, or to the event list when no other pane of its window can take it.
pub(crate) fn toggle(a: &mut App, view: ViewKind) {
    let pane = pane(&focus::root(a), view);
    let views = pane.views();
    if !folded(a, view) && views.contains(&focus::focused(a)) {
        focus::step(a, true);
        if views.contains(&focus::focused(a)) {
            focus::set(a, ViewKind::EventList);
        }
    }
    a.pane_collapse.toggle(&pane);
}

/// Opens `view`'s pane if it is folded.
pub(crate) fn unfold(a: &mut App, view: ViewKind) {
    if folded(a, view) {
        let pane = pane(&focus::root(a), view);
        a.pane_collapse.toggle(&pane);
    }
}

/// The header line's height, which is also the width of a pane folded inside a row.
pub(crate) fn thickness(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y
}

/// The side a pane folds toward. Inside a row the open panes share the freed width, so the last
/// pane narrows toward the window's right edge and the others toward their left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Top,
    Left,
    Right,
}

impl Side {
    /// The side of child `index` among `count` laid out along `axis`.
    pub(crate) fn of(axis: Axis, index: usize, count: usize) -> Self {
        match axis {
            Axis::Column => Self::Top,
            Axis::Row if index + 1 == count => Self::Right,
            Axis::Row => Self::Left,
        }
    }
}

/// Draws the top of `view`'s pane, which folds toward `side`: a line with the toggle and
/// `title`, or the view's name once folded, or the toggle alone when folded inside a row.
/// `true` while it is open.
pub(crate) fn header(
    a: &mut App,
    ui: &mut egui::Ui,
    view: ViewKind,
    side: Side,
    title: impl FnOnce(&mut egui::Ui),
) -> bool {
    let open = !folded(a, view);
    let size = thickness(ui);
    let mut rect = ui.available_rect_before_wrap();
    if open || side == Side::Top {
        rect.max.y = rect.min.y + size;
    }
    // Register the background before its children so the toggle and tabs keep their clicks.
    let background = ui.interact(rect, ui.id().with("pane-header"), egui::Sense::click());
    let clicked = if !open && side != Side::Top {
        toggle_button(ui, open, side, size)
            .on_hover_text(format!(
                "{}
{}",
                view.name(),
                hint(open)
            ))
            .clicked()
    } else {
        ui.horizontal(|ui| {
            ui.set_height(size);
            // Header names belong to the background action rather than text selection.
            ui.style_mut().interaction.selectable_labels = false;
            let clicked = toggle_button(ui, open, side, size)
                .on_hover_text(hint(open))
                .clicked();
            if open {
                title(ui);
            } else {
                ui.label(view.name());
            }
            clicked
        })
        .inner
    };
    if clicked || (!open && background.clicked()) {
        toggle(a, view);
    } else if open && background.clicked() && focus::takes_focus(view) {
        focus::set(a, view);
    }
    !folded(a, view)
}

fn hint(open: bool) -> &'static str {
    if open {
        "折りたたむ"
    } else {
        "折りたたみを解除する"
    }
}

/// The chevron points the way the pane goes: down and right inside a column as in a tree view,
/// toward `side` and back inside a row.
fn toggle_button(ui: &mut egui::Ui, open: bool, side: Side, size: f32) -> egui::Response {
    use std::f32::consts::TAU;
    let (_, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let angle = match (open, side) {
        (true, Side::Top) => 0.0,
        (true, Side::Left) | (false, Side::Right) => TAU / 4.0,
        (false, Side::Top) | (true, Side::Right) | (false, Side::Left) => -TAU / 4.0,
    };
    paint_chevron(ui, angle, &response);
    response
}

/// Thicker than egui's own strokes, so the chevron stands apart from a checkbox's tick.
pub(crate) const CHEVRON_WIDTH: f32 = 1.75;

/// An open stroked chevron pointing down at `angle` zero, unlike the filled triangle of play.
fn paint_chevron(ui: &egui::Ui, angle: f32, response: &egui::Response) {
    let visuals = ui.style().interact(response);
    let half = response.rect.height() * 0.2 + visuals.expansion;
    let center = response.rect.center();
    let rotation = egui::emath::Rot2::from_angle(angle);
    let points = [
        egui::vec2(-half, -half * 0.5),
        egui::vec2(0.0, half * 0.5),
        egui::vec2(half, -half * 0.5),
    ]
    .map(|v| center + rotation * v)
    .to_vec();
    ui.painter().add(egui::Shape::line(
        points,
        egui::Stroke::new(CHEVRON_WIDTH, visuals.fg_stroke.color),
    ));
}

#[cfg(test)]
mod tests;
