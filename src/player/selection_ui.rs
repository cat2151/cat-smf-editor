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
    egui::Window::new("Player selection")
        .open(&mut app.player_selection.open)
        .show(ctx, |ui| {
            if let Some(error) = &app.player_selection.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            ui.hyperlink_to("Download cat-plugin-player", DOWNLOAD);
            ui.label("Executable: --player <path>, beside the editor, sibling repo target/release, or PATH.");
        });
}
