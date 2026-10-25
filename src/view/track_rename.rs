use crate::app::App;
use eframe::egui::{self, Event, ImeEvent};

pub struct Editing {
    song: u64,
    generation: u64,
    track: usize,
    text: String,
    focus: bool,
    composing: bool,
    error: Option<String>,
}

pub fn begin(app: &mut App, track: usize) {
    let Some(source) = app.loaded.as_ref().and_then(|l| l.music.tracks.get(track)) else {
        return;
    };
    app.track_rename = Some(Editing {
        song: app.song_id,
        generation: app.generation,
        track,
        text: source.name().unwrap_or_default(),
        focus: true,
        composing: false,
        error: None,
    });
    app.track_view.go_open = false;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app
        .track_rename
        .as_ref()
        .is_some_and(|e| e.song != app.song_id || e.generation != app.generation)
    {
        app.track_rename = None;
    }
    if super::commands::owner(app, ctx) != super::commands::Screen::TrackRename {
        return;
    }
    let Some(mut editing) = app.track_rename.take() else {
        return;
    };
    let mut ime = editing.composing;
    ctx.input(|i| {
        for event in &i.events {
            match event {
                Event::Ime(ImeEvent::Preedit(text)) => {
                    editing.composing = !text.is_empty();
                    ime |= editing.composing;
                }
                Event::Ime(ImeEvent::Commit(_)) => {
                    editing.composing = false;
                    ime = true;
                }
                Event::Ime(ImeEvent::Disabled) => editing.composing = false,
                _ => {}
            }
        }
    });
    if ime {
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: egui::Key::Enter,
                        ..
                    }
                )
            })
        });
    }
    let mut confirm =
        !ime && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let mut cancel =
        !ime && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    egui::Modal::new(egui::Id::new("track-rename")).show(ctx, |ui| {
        ui.heading(format!("Track {} · Rename", editing.track + 1));
        let mut output = egui::TextEdit::singleline(&mut editing.text)
            .desired_width(320.0)
            .show(ui);
        if editing.focus {
            output.response.request_focus();
            output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(editing.text.chars().count()),
                )));
            output.state.store(ctx, output.response.id);
            editing.focus = false;
        }
        ui.horizontal(|ui| {
            confirm |= ui.button("Rename · Enter").clicked();
            cancel |= ui.button("Cancel · Esc").clicked();
        });
        if let Some(error) = &editing.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    });
    if cancel {
        return;
    }
    if confirm {
        match app.edit_music(|music| {
            let track = music
                .tracks
                .get_mut(editing.track)
                .ok_or("Track was removed")?;
            if track.name().unwrap_or_default() == editing.text {
                return Ok(());
            }
            cat_smf_editor::edit::tracks::rename(track, &editing.text)
        }) {
            Ok(()) => return,
            Err(error) => editing.error = Some(error),
        }
    }
    app.track_rename = Some(editing);
    super::comma_menu::swallow(ctx);
}

#[cfg(test)]
mod tests;
