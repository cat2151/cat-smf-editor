//! Bound surfaces open the whole container chain and regain their keys on close.
use super::{open, Target};
use crate::{
    app::App,
    view::dock::{focus, locate, ViewKind},
};
use eframe::egui;

pub(crate) fn sequencer(app: &mut App) {
    let viewport = locate::viewport_of(app, ViewKind::StepSequencer);
    // Keep the grid's target, cursor and loop while its modal chain is edited.
    let grid = std::mem::take(&mut app.drum_editor);
    bound(app, app.track, ViewKind::StepSequencer, viewport);
    app.drum_editor = grid;
}

pub(crate) fn bound(app: &mut App, track: usize, view: ViewKind, viewport: egui::ViewportId) {
    open(app, Target::Track(track), usize::MAX);
    app.effect_editor.return_to = Some((view, viewport));
}

pub(super) fn hand_back(app: &mut App, ctx: &egui::Context) {
    if let Some((view, viewport)) = app.effect_editor.return_to.take() {
        if focus::placed(app, view) {
            focus::set(app, view);
        }
        crate::view::presentation::focus_after_restoration(ctx, viewport);
    }
}
