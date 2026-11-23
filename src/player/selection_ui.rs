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
    crate::view::presentation::window(
        ctx,
        crate::view::commands::Screen::Browser,
        egui::Id::new("Player selection"),
        "Player selection",
    )
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

#[cfg(windows)]
pub(crate) fn show_loading(app: &App, ctx: &egui::Context) {
    use crate::player::resident_link;
    use crate::view::{presentation, song_challenge};

    if !resident_link::loading(app) {
        return;
    }
    let id = egui::Id::new("browser-loading");
    let challenge_browser = resident_link::target(&app.player_selection)
        .is_some_and(|target| song_challenge::awaiting_browser(app, target));
    let modal = if challenge_browser {
        // The external browser takes focus as it becomes visible.
        presentation::viewport_modal(id)
    } else {
        presentation::modal(ctx, id)
    };
    modal.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("now loading...");
        });
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}
