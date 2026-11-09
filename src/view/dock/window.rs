//! Dock windows: native windows split into panes like the main window. Their layout lives in
//! `layout.toml`, so closing one does nothing.
use super::{focus, Node, ViewKind};
use crate::app::App;
use crate::view::patterns::workspace::Pane;
use crate::view::presentation;
use eframe::egui;

pub(crate) fn viewport(index: usize) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("dock-window", index))
}

/// The dock window behind `id`, if any.
pub(crate) fn index(a: &App, id: egui::ViewportId) -> Option<usize> {
    (0..a.layout.spec.docks.len()).find(|&index| viewport(index) == id)
}

pub(crate) fn show(a: &mut App, ctx: &egui::Context) {
    for (index, dock) in focus::root(a).docks.into_iter().enumerate() {
        let id = viewport(index);
        let views = dock.node.views();
        if !presentation::Native::Dock.visible(a, ctx, id) {
            for view in &views {
                suspend(a, ctx, *view);
            }
        }
        let target = a.layout.resolved.get(index).copied().flatten();
        let [x, y, width, height] = match &dock.placement {
            crate::store::layout::placement::Placement::Desktop(rect) => *rect,
            crate::store::layout::placement::Placement::Monitor(_) => {
                let Some(target) = target else {
                    continue;
                };
                target.outer.map(|n| n as f32 / target.scale)
            }
        };
        let builder = egui::ViewportBuilder::default()
            .with_title(&dock.title)
            .with_active(false)
            .with_position([x, y])
            .with_inner_size([width, height]);
        presentation::Native::Dock.show(a, ctx, id, builder, |a, ctx, class| {
            let embedded = class == egui::ViewportClass::Embedded;
            if !embedded && let Some(target) = target {
                crate::store::layout::monitors::apply(ctx, &dock.title, target);
            }
            if !embedded && ctx.input(|i| i.events.iter().any(pressed)) {
                focus::claim(a, index);
            }
            let input = crate::view::modal::begin(a, ctx);
            crate::view::commands::route(a, ctx);
            presentation::layers(a, ctx);
            if embedded {
                egui::Window::new(&dock.title)
                    .id(egui::Id::new(("dock-window", index)))
                    .default_pos([x, y])
                    .default_size([width, height])
                    .show(ctx, |ui| panes(a, ui, index, &dock.node));
            } else {
                egui::CentralPanel::default().show(ctx, |ui| panes(a, ui, index, &dock.node));
            }
            for view in &views {
                finish(a, ctx, *view);
            }
            crate::view::patterns::workspace::host_dialog(a, ctx);
            crate::play::playback::show(a, ctx);
            crate::view::modal::end(a, ctx, input);
            // Only layout.toml removes this window, so its × quits the app.
            if crate::view::alt_f4::closes(ctx) {
                a.request_close(ctx);
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        });
    }
}

/// As in the main window, a menu or dialog over the panes leaves them disabled.
fn panes(a: &mut App, ui: &mut egui::Ui, index: usize, node: &Node) {
    if !presentation::panes_enabled(a, ui.ctx(), super::Host::Dock(index)) {
        ui.disable();
    }
    crate::view::score_panes::dock(a, ui, node);
}

fn pressed(event: &egui::Event) -> bool {
    matches!(event, egui::Event::Key { pressed: true, .. })
}

/// The work the main window does for its own panes after drawing them.
fn finish(a: &mut App, ctx: &egui::Context, view: ViewKind) {
    match view {
        ViewKind::PianoRoll => crate::view::piano_roll::run(a, ctx),
        ViewKind::TempoCurve => crate::view::tempo_curve::run(a, ctx),
        ViewKind::Mixer => crate::play::mixer::run(a, ctx),
        ViewKind::EventList
        | ViewKind::Score
        | ViewKind::Analysis
        | ViewKind::Song
        | ViewKind::Scene
        | ViewKind::Keyboard
        | ViewKind::Piano
        | ViewKind::Help
        | ViewKind::StepSequencer
        | ViewKind::ParameterCurve
        | ViewKind::TrackOverview => {}
    }
}

/// Drops the gestures of `view`, which is not drawn this frame.
pub(crate) fn suspend(a: &mut App, ctx: &egui::Context, view: ViewKind) {
    match view {
        ViewKind::PianoRoll => crate::view::piano_roll::suspend(a, ctx),
        ViewKind::TempoCurve => crate::view::tempo_curve::suspend(a),
        ViewKind::Mixer => a.undo.end_volume_gesture(),
        ViewKind::Score => crate::view::staff::suspend(a),
        ViewKind::Song => crate::view::patterns::workspace::suspend(a, Pane::Song),
        ViewKind::Scene => crate::view::patterns::workspace::suspend(a, Pane::Scene),
        ViewKind::StepSequencer => crate::view::drums::suspend(a),
        ViewKind::ParameterCurve => crate::view::parameter_editor::suspend(a),
        ViewKind::TrackOverview => crate::view::track_overview::suspend(a),
        ViewKind::EventList
        | ViewKind::Analysis
        | ViewKind::Keyboard
        | ViewKind::Piano
        | ViewKind::Help => {}
    }
}

#[cfg(test)]
mod tests;
