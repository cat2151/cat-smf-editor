//! Slow actions run one pass after they are requested, so the modal is painted
//! before the UI thread blocks.
use crate::app::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    New,
    AddTrack,
    MmlInput,
    MmlMeasure,
    MmlConfirm,
    Close,
}

#[derive(Default)]
pub struct State {
    /// The task and the pass that requested it.
    task: Option<(Task, u64)>,
    mml_playback: bool,
}

pub fn request(app: &mut App, ctx: &egui::Context, task: Task) {
    app.busy.task = Some((task, ctx.cumulative_pass_nr()));
    if task == Task::MmlConfirm {
        app.busy.mml_playback = true;
    }
    ctx.request_repaint();
}

pub fn pending(app: &App) -> bool {
    app.busy.task.is_some()
}

pub fn run(app: &mut App, ctx: &egui::Context) {
    if !pending(app) && !crate::play::transport::preparing(app) {
        app.busy.mml_playback = false;
    }
    let Some((task, pass)) = app.busy.task else {
        return;
    };
    // Layout retries are still the same presented frame.
    if ctx.cumulative_pass_nr() <= pass
        || task == Task::MmlConfirm && ctx.output(|o| o.num_completed_passes > 0)
    {
        ctx.request_repaint();
        return;
    }
    app.busy.task = None;
    match task {
        Task::New => app.new_smf_unchecked(),
        Task::AddTrack => {
            if let Err(error) = app.add_track() {
                app.load_error = Some(error);
            }
        }
        Task::MmlInput => super::mml_input::open(app, ctx, false),
        Task::MmlMeasure => super::mml_input::open(app, ctx, true),
        Task::MmlConfirm => super::mml_input::ready_to_confirm(app),
        Task::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
    }
}

/// Also covers the shutdown after a close is approved. Painted on a plain layer:
/// an `Area` spends its first pass invisible and then fades in.
pub fn show(app: &App, ctx: &egui::Context) {
    let playback =
        app.busy.mml_playback && (pending(app) || crate::play::transport::preparing(app));
    if !pending(app) && !playback && !app.config.navigation.close_approved {
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
    if playback {
        ui.painter().text(
            screen.center() + egui::vec2(0.0, SPINNER),
            egui::Align2::CENTER_CENTER,
            "Preparing playback, please wait...",
            egui::FontId::proportional(20.0),
            egui::Color32::WHITE,
        );
        ctx.request_repaint();
    }
}

const SPINNER: f32 = 48.0;

#[cfg(test)]
mod tests;
