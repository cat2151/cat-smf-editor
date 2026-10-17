use crate::app::App;
use eframe::egui;

pub(super) fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if (ctx.wants_keyboard_input() && !app.playback.mixer_focused() && !app.layer_editor.open)
        || ctx.input(|i| i.pointer.any_down() || i.pointer.any_released())
        || app.loaded.is_none()
        || app.recent.open
        || !app.layer_editor.allows_undo()
        || app.note_edit.is_some()
        || app.trill_edit.is_some()
        || app.help_open
        || app.player_selection.open
        || app.playback.open
        || app.comma_menu.open
        || app.edit_menu.open
        || app.track_view.go_open
        || app.track_view.split_open
        || app.config.navigation.departure.is_some()
    {
        return;
    }
    let pressed = ctx.input_mut(|i| {
        let pressed = i.events.iter().any(|e| {
            matches!(e, egui::Event::Key {
                key: egui::Key::Z, pressed: true, repeat: false, modifiers, ..
            } if *modifiers == egui::Modifiers::CTRL)
        });
        if pressed {
            i.consume_key(egui::Modifiers::CTRL, egui::Key::Z);
        }
        pressed
    });
    if pressed {
        apply(app);
        super::comma_menu::swallow(ctx);
    }
}

pub(super) fn apply(app: &mut App) {
    if let Err(error) = app.undo_music() {
        app.load_error = Some(error);
    }
}
