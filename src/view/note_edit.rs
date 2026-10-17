//! Typing over a Note On's MBT or Duration cell moves only that note's start or end.
use crate::app::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Mbt,
    Duration,
}

impl Field {
    /// The Notes view column of this field.
    pub fn column(self) -> usize {
        match self {
            Field::Mbt => 0,
            Field::Duration => 2,
        }
    }

    pub fn at(column: usize) -> Option<Self> {
        [Field::Mbt, Field::Duration]
            .into_iter()
            .find(|field| field.column() == column)
    }
}

pub struct Editing {
    field: Field,
    track: usize,
    event: usize,
    generation: u64,
    rect: egui::Rect,
    text: String,
    error: Option<String>,
    focus: bool,
}

pub fn begin(
    app: &mut App,
    field: Field,
    track: usize,
    event: usize,
    rect: egui::Rect,
    text: &str,
) {
    app.note_edit = Some(Editing {
        field,
        track,
        event,
        generation: app.generation,
        rect,
        text: text.split_whitespace().collect(),
        error: None,
        focus: true,
    });
}

fn apply(app: &mut App, editing: &Editing) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let (track, event) = (editing.track, editing.event);
    cat_smf_editor::edit::trill::ensure_note_editable(&loaded.music, track, event)?;
    match editing.field {
        Field::Mbt => {
            let tick = crate::store::session::parse_mbt(&editing.text)
                .and_then(|position| loaded.time.map(track)?.tick_at(&position))
                .ok_or("Invalid MBT")?;
            app.edit_music(|music| {
                cat_smf_editor::edit::note_start::move_start(&mut music.tracks[track], event, tick)
            })
        }
        Field::Duration => {
            let duration = editing
                .text
                .parse::<u64>()
                .map_err(|_| "Invalid duration".to_string())?;
            app.edit_music(|music| {
                cat_smf_editor::edit::note_length::set_duration(
                    &mut music.tracks[track],
                    event,
                    duration,
                )
            })
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut editing) = app.note_edit.take() else {
        return;
    };
    if editing.generation != app.generation {
        return;
    }
    let mut close = false;
    egui::Area::new(egui::Id::new("note-edit"))
        .fixed_pos(editing.rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut editing.text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(editing.rect.width()),
            );
            if editing.focus {
                response.request_focus();
                editing.focus = false;
            }
            if response.lost_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    match apply(app, &editing) {
                        Ok(()) => close = true,
                        Err(error) => {
                            editing.error = Some(error);
                            editing.focus = true;
                        }
                    }
                } else {
                    // Esc or a click elsewhere cancels.
                    close = true;
                }
            }
            if let Some(error) = &editing.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
    if !close {
        app.note_edit = Some(editing);
    }
}

#[cfg(test)]
pub(crate) mod tests;
