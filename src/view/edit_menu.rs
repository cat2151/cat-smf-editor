//! `e` opens the note editor for the cursor's note; `.` repeats the last `e` command.
use crate::{
    app::App,
    view::{
        comma_menu::{pressed, swallow},
        note_edit::Field,
    },
};
use eframe::egui;

const CHOICES: [(egui::Key, Field, &str); 2] = [
    (egui::Key::M, Field::Mbt, "m: MBT"),
    (egui::Key::D, Field::Duration, "d: Duration"),
];

/// A command remembered up to its number field; the number itself is typed again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Note(Field),
}

#[derive(Default)]
pub struct State {
    pub open: bool,
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    if app.comma_menu.open
        || app.layer_editor.open
        || app.note_edit.is_some()
        || app.trill_edit.is_some()
        || app.track_view.go_open
        || app.track_view.split_open
        || app.help_open
        || app.playback.open
        || app.playback.mixer_focused()
        || app.player_selection.open
        || app.loaded.is_none()
        || ctx.wants_keyboard_input()
    {
        return;
    }
    if !app.edit_menu.open {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::E)) {
            app.edit_menu.open = true;
            swallow(ctx);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Period)) {
            repeat(app);
            swallow(ctx);
        }
        return;
    }
    for key in pressed(ctx) {
        if key == egui::Key::Escape {
            app.edit_menu.open = false;
        } else if key == egui::Key::U {
            app.edit_menu.open = false;
            super::undo::apply(app);
        } else if let Some(&(_, field, _)) = CHOICES.iter().find(|(k, ..)| *k == key) {
            choose(app, field);
        }
    }
    // The menu owns the keyboard while open, so its letters never reach other shortcuts.
    swallow(ctx);
}

fn choose(app: &mut App, field: Field) {
    app.edit_menu.open = false;
    app.last_command = Some(Command::Note(field));
    edit_cursor_note(app, field);
}

/// The row drawing the cursor's note opens the field, so the field sits on its cell.
fn edit_cursor_note(app: &mut App, field: Field) {
    app.note_edit_request = Some(field);
    app.reveal_cursor = true;
}

fn repeat(app: &mut App) {
    match app.last_command {
        Some(Command::Note(field)) => edit_cursor_note(app, field),
        None => {}
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.edit_menu.open {
        return;
    }
    let mut open = true;
    egui::Window::new("e")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            if ui
                .add_enabled(app.undo.available(), egui::Button::new("u: Undo"))
                .clicked()
            {
                app.edit_menu.open = false;
                super::undo::apply(app);
            }
            for (_, field, label) in CHOICES {
                if ui.button(label).clicked() {
                    choose(app, field);
                }
            }
        });
    if !open {
        app.edit_menu.open = false;
    }
}

#[cfg(test)]
mod tests;
