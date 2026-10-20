//! Typing over a Note On's MBT or Duration cell moves only that note's start or end;
//! its lyric is typed over the Note cell.
use crate::app::App;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Mbt,
    Duration,
    Lyrics,
}

impl Field {
    /// The Notes view column of this field.
    pub fn column(self) -> usize {
        match self {
            Field::Mbt => 0,
            Field::Lyrics => 1,
            Field::Duration => 2,
        }
    }

    /// The field a click on `column` opens; the lyric opens only from the edit menu.
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
        text: if field == Field::Lyrics {
            text.to_owned()
        } else {
            text.split_whitespace().collect()
        },
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
        Field::Lyrics => {
            let note = &loaded.music.tracks[track].events[event];
            let (tick, identity) = (note.tick, (note.status, note.data.clone()));
            let occurrence = same_notes(&loaded.music.tracks[track].events, tick, &identity)
                .position(|i| i == event)
                .unwrap_or(0);
            app.edit_music(|music| {
                cat_smf_editor::edit::lyric::set(
                    &mut music.tracks[track],
                    event,
                    editing.text.trim(),
                )
            })?;
            keep_cursor_on(app, track, tick, &identity, occurrence);
            crate::play::sung::prerender::ahead(app, track);
            Ok(())
        }
    }
}

fn same_notes<'a>(
    events: &'a [cat_smf_editor::model::Event],
    tick: u64,
    identity: &'a (u8, Vec<u8>),
) -> impl Iterator<Item = usize> + 'a {
    (0..events.len()).filter(move |&i| {
        let e = &events[i];
        e.tick == tick && e.status == identity.0 && e.data == identity.1
    })
}

/// Adding or removing the lyric row shifts the note's row; keep the cursor on the note so `j` leaves it.
fn keep_cursor_on(
    app: &mut App,
    track: usize,
    tick: u64,
    identity: &(u8, Vec<u8>),
    occurrence: usize,
) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    if app.track != track || !app.note_mode {
        return;
    }
    let Some(event) =
        same_notes(&loaded.music.tracks[track].events, tick, identity).nth(occurrence)
    else {
        return;
    };
    if let Some(row) = loaded.display_rows[track].event_row(event) {
        app.cursor_row = Some(row);
        app.selected_event = Some(event);
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.note_edit.is_some() {
        crate::view::commands::route(app, ctx);
    }
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
            if response.lost_focus() && !app.palette.is_open() {
                close = true;
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
mod palette_tests;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) fn confirm(app: &mut App, _ctx: &egui::Context) {
    if let Some(mut editing) = app.note_edit.take()
        && let Err(error) = apply(app, &editing)
    {
        editing.error = Some(error);
        editing.focus = true;
        app.note_edit = Some(editing);
    }
}
