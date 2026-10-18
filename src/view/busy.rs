//! Slow actions run one pass after they are requested, so the modal is painted
//! before the UI thread blocks.
use crate::app::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    New,
    MmlInput,
    Close,
}

#[derive(Default)]
pub struct State {
    /// The task and the pass that requested it.
    task: Option<(Task, u64)>,
}

pub fn request(app: &mut App, ctx: &egui::Context, task: Task) {
    app.busy.task = Some((task, ctx.cumulative_pass_nr()));
    ctx.request_repaint();
}

pub fn pending(app: &App) -> bool {
    app.busy.task.is_some()
}

pub fn run(app: &mut App, ctx: &egui::Context) {
    let Some((task, pass)) = app.busy.task else {
        return;
    };
    if ctx.cumulative_pass_nr() <= pass {
        return;
    }
    app.busy.task = None;
    match task {
        Task::New => app.new_smf_unchecked(),
        Task::MmlInput => super::mml_input::open(app, ctx),
        Task::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
    }
}

/// Also covers the shutdown after a close is approved. Painted on a plain layer:
/// an `Area` spends its first pass invisible and then fades in.
pub fn show(app: &App, ctx: &egui::Context) {
    if !pending(app) && !app.config.navigation.close_approved {
        return;
    }
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("busy"));
    let screen = ctx.screen_rect();
    let ui = egui::Ui::new(
        ctx.clone(),
        layer.id,
        egui::UiBuilder::new().layer_id(layer).max_rect(screen),
    );
    ui.painter()
        .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
    egui::Spinner::new().size(SPINNER).paint_at(
        &ui,
        egui::Rect::from_center_size(screen.center(), egui::Vec2::splat(SPINNER)),
    );
}

const SPINNER: f32 = 48.0;

#[cfg(test)]
mod tests;
