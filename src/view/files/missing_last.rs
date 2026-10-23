use crate::app::App;
use eframe::egui;

/// Owns input until the user picks a new song or quits.
pub(crate) fn show(app: &mut App, ctx: &egui::Context) -> bool {
    if app.missing_last_mid.is_none() {
        return false;
    }
    crate::view::commands::route(app, ctx);
    let Some(path) = &app.missing_last_mid else {
        return true;
    };
    let mut create = false;
    let mut quit = false;
    egui::Modal::new(egui::Id::new("missing-last-mid")).show(ctx, |ui| {
        ui.set_max_width(480.0);
        ui.strong(path.display().to_string());
        ui.label("前回開いていたファイルが見つかりません");
        create |= ui.button("n: 新規SMF").clicked();
        quit |= ui.button("q: アプリ終了").clicked();
    });
    super::super::comma_menu::swallow(ctx);
    if create {
        create_new_song(app);
    } else if quit {
        app.request_close(ctx);
    }
    true
}

pub(crate) fn create_new_song(app: &mut App) {
    app.missing_last_mid = None;
    app.new_smf_unchecked();
    crate::view::analysis::restore(app);
}
