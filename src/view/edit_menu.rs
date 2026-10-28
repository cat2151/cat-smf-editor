//! `e` opens the note editor for the cursor's note; `.` repeats the last `e` command.
use crate::{app::App, view::note_edit::Field};
use eframe::egui;

const CHOICES: [(egui::Key, Field, &str); 5] = [
    (egui::Key::P, Field::Pitch, "p: Pitch"),
    (egui::Key::V, Field::Velocity, "v: Velocity"),
    (egui::Key::M, Field::Mbt, "m: MBT"),
    (egui::Key::D, Field::Duration, "d: Duration"),
    (egui::Key::L, Field::Lyrics, "l: Lyrics"),
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
    crate::view::commands::route(app, ctx);
}

pub(crate) fn choose(app: &mut App, field: Field) {
    app.edit_menu.open = false;
    app.last_command = Some(Command::Note(field));
    edit_cursor_note(app, field);
}

/// The row drawing the cursor's note opens the field, so the field sits on its cell.
fn edit_cursor_note(app: &mut App, field: Field) {
    if field == Field::Lyrics {
        step_off_lyric_row(app);
    }
    app.note_edit_request = Some(field);
    app.reveal_cursor = true;
}

/// A lyric row sits just above its note, so `j` from the previous note stops on it;
/// the lyric there belongs to the next Note On at that tick.
fn step_off_lyric_row(app: &mut App) {
    let (Some(loaded), Some(selected)) = (&app.loaded, app.selected_event) else {
        return;
    };
    let events = &loaded.visible().tracks[app.track].events;
    let lyric = events
        .get(selected)
        .is_some_and(|e| e.status == 0xff && e.meta_type == cat_smf_editor::edit::lyric::LYRIC);
    if !lyric || !app.note_mode {
        return;
    }
    let tick = events[selected].tick;
    let Some(note) = (selected + 1..events.len())
        .take_while(|&i| events[i].tick == tick)
        .find(|&i| cat_smf_editor::score::note_view::note_on(&events[i]))
    else {
        return;
    };
    if let Some(row) = loaded.display_rows[app.track].event_row(note) {
        app.cursor_row = Some(row);
        app.selected_event = Some(note);
    }
}

pub(crate) fn repeat(app: &mut App) {
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
pub(crate) mod lyric_tests;
#[cfg(test)]
mod tests;
