use crate::app::App;
use eframe::egui;

pub(super) fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

pub(super) fn apply(app: &mut App) {
    if let Err(error) = app.undo_music() {
        app.load_error = Some(error);
    }
}
