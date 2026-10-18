use super::DOWNLOAD;
use crate::app::App;
use eframe::egui;

pub fn show(app: &mut App, ctx: &egui::Context) {
    #[cfg(windows)]
    crate::player::resident_link::poll(app, ctx);
    if app.player_selection.no_notes_error {
        crate::player::bulk::show_note_error(app, ctx);
        return;
    }
    if !app.player_selection.open {
        return;
    }
    let mut setup = false;
    egui::Window::new("Player selection")
        .open(&mut app.player_selection.open)
        .show(ctx, |ui| {
            if let Some(error) = &app.player_selection.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            ui.hyperlink_to("Download cat-plugin-player", DOWNLOAD);
            setup = ui.button("Setup").clicked();
        });
    if setup {
        app.player_selection.open = false;
        app.player_selection
            .setup
            .begin(app.player_selection.executable.clone(), ctx);
    }
}
