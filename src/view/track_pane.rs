use crate::app::App;
use crate::view::note_edit::Field;
use crate::view::row::{beat_color, raw_preview, row, RowStyle};
use cat_smf_editor::score::display_rows::DisplayRow;
use cat_smf_editor::score::note_view::note_on;
use eframe::egui::{self, ScrollArea};

mod content;
mod header;

pub fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    track: usize,
    active: bool,
    browse_patch: &mut bool,
    replace_effect: &mut Option<usize>,
) {
    let selection = (track == app.track)
        .then(|| super::event_selection::range(app))
        .flatten();
    let marker = crate::play::transport::marker_for(app, track);
    let preparing = crate::play::transport::preparing_row_for(app, track);
    let current_tick = crate::play::playback::location(app).0;
    let Some(loaded) = &app.loaded else {
        ui.label("—");
        return;
    };
    let map = loaded.time.map(track);
    let keys = loaded.keys.map(track);
    let (patch, effect) = header::show(loaded, ui, track);
    let events = &loaded.music.tracks[track].events;
    let notes = &loaded.note_views[track];
    let columns = &loaded.time_columns[track];
    let display = &loaded.display_rows[track];
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let char_width = ui.fonts(|fonts| fonts.glyph_width(&font, '0'));
    let chars = |n: usize| n as f32 * char_width;
    let widths: Vec<f32> = if app.note_mode {
        vec![
            55.0,
            (chars(columns.delta) + 8.0).max(60.0),
            (chars(columns.absolute) + 8.0).max(75.0),
            (chars(columns.position_chars()) + 8.0).max(90.0),
            430.0,
        ]
    } else {
        // Event, Delta, Tick, MBT, Raw, then the note view's Note, Duration, Velocity.
        vec![
            chars(events.len().to_string().len()),
            chars(columns.delta),
            chars(columns.absolute),
            chars(columns.position_chars()),
            chars(columns.raw),
            chars(4),
            chars(columns.absolute),
            chars(3),
        ]
    };
    let row_count = if app.note_mode {
        display.len()
    } else {
        events.len()
    };
    let mut repeat = None;
    let mut selecting_repeat = false;
    let mut selected = None;
    let mut begin_edit = None;
    let mut trill_click = None;
    let horizontal_input = ui.input(|i| i.smooth_scroll_delta.x != 0.0 || i.pointer.any_down());
    let horizontal = ScrollArea::horizontal()
        .id_salt(("event-horizontal", track))
        .drag_to_scroll(false)
        .horizontal_scroll_offset(app.track_view.horizontal_offset)
        .show(ui, |ui| {
            ui.set_min_width(if app.note_mode {
                widths[3] + 726.0
            } else {
                widths.iter().sum::<f32>()
                    + crate::view::row::raw_spacing(ui) * widths.len() as f32
                    + 16.0
            });
            ui.spacing_mut().item_spacing.y = 1.0;
            let headers: &[&str] = if app.note_mode {
                &["MBT", "Note", "Duration", "Velocity"]
            } else {
                &[
                    "Event", "Delta", "Tick", "MBT", "Raw", "Note", "Duration", "Velocity",
                ]
            };
            row(
                ui,
                headers,
                RowStyle {
                    note_mode: app.note_mode,
                    selected: false,
                    playing: false,
                    preparing: false,
                    boundary: None,
                    beat_head: false,
                    position: None,
                    raw_event: None,
                    pitches: &[],
                    key: None,
                    pitch_range: columns.pitch_range,
                    widths: &widths,
                    caution: None,
                    spans: None,
                },
            );
            let height = (ui.text_style_height(&egui::TextStyle::Body) + 2.0)
                .max(ui.spacing().interact_size.y);
            let mut scroll = ScrollArea::vertical()
                .id_salt(("events", app.generation, track, app.note_mode))
                .drag_to_scroll(false)
                .auto_shrink([false, false]);
            let stride = height + ui.spacing().item_spacing.y;
            let viewport = ui.available_height().max(height);
            // Every pane keeps its own cursor on the vertical center, so split panes line up.
            let center = |row: f32| (row * stride - viewport * 0.5).max(0.0);
            let focus = marker
                .or(preparing)
                .filter(|_| app.playback.follow)
                .or_else(|| {
                    app.cursor_row.filter(|_| app.reveal_cursor).map(|cursor| {
                        if active {
                            cursor
                        } else {
                            crate::cursor::split::row_sounding_loaded(
                                loaded,
                                app.note_mode,
                                track,
                                current_tick,
                            )
                        }
                    })
                });
            if let Some(row) = focus {
                scroll = scroll.vertical_scroll_offset(center(row as f32 + 0.5));
            } else if app.reset_scroll {
                scroll = scroll.vertical_scroll_offset(0.0);
            } else if app.track_view.split.is_some() || app.track_view.apply_scroll {
                let r = app
                    .track_view
                    .scroll_source
                    .filter(|(source, _)| *source == track)
                    .map_or_else(
                        || {
                            crate::cursor::split::row_sounding_loaded(
                                loaded,
                                app.note_mode,
                                track,
                                app.track_view.scroll_tick,
                            )
                        },
                        |(_, row)| row,
                    );
                scroll = scroll
                    .vertical_scroll_offset(center(r as f32 + app.track_view.scroll_fraction));
            }
            // The ScrollArea consumes the wheel delta, so sample it before showing rows.
            let input_active = ui.input(|i| i.smooth_scroll_delta.y != 0.0 || i.pointer.any_down());
            let output = scroll.show_rows(ui, height, row_count, |ui, range| {
                for visible_index in range {
                    let index = if app.note_mode {
                        match display.get(visible_index) {
                            DisplayRow::Event(index) => index,
                            DisplayRow::Beat(position) => {
                                let mbt = columns
                                    .rest_position(&position, display.is_measure(visible_index));
                                if row(
                                    ui,
                                    &[&mbt, ""],
                                    RowStyle {
                                        note_mode: true,
                                        selected: active && app.cursor_row == Some(visible_index),
                                        playing: marker == Some(visible_index),
                                        preparing: preparing == Some(visible_index),
                                        boundary: Some(egui::Stroke::new(
                                            if position.beat == 1 { 1.5_f32 } else { 0.7_f32 },
                                            beat_color(position.beat).gamma_multiply(0.65),
                                        )),
                                        beat_head: true,
                                        position: Some(&position),
                                        raw_event: None,
                                        pitches: &[],
                                        key: None,
                                        pitch_range: columns.pitch_range,
                                        widths: &widths,
                                        caution: None,
                                        spans: None,
                                    },
                                )
                                .clicked
                                .is_some()
                                {
                                    selected = Some(visible_index);
                                }
                                continue;
                            }
                        }
                    } else {
                        visible_index
                    };
                    let event = &events[index];
                    let position = map.map(|m| m.position(event.tick));
                    // Compare visible rows, even at the viewport edge.
                    let position_at = |i| {
                        if app.note_mode {
                            match display.get(i) {
                                DisplayRow::Event(i) => map.unwrap().position(events[i].tick),
                                DisplayRow::Beat(p) => p,
                            }
                        } else {
                            map.unwrap().position(events[i].tick)
                        }
                    };
                    let beat_head = position.as_ref().is_some_and(|current| {
                        let mut first = visible_index;
                        while let Some(i) = first.checked_sub(1)
                            && position_at(i) == *current
                        {
                            first = i;
                        }
                        first.checked_sub(1).is_none_or(|i| {
                            let previous = position_at(i);
                            (previous.measure, previous.beat) != (current.measure, current.beat)
                        })
                    });
                    let boundary = position.as_ref().and_then(|current| {
                        let previous = visible_index.checked_sub(1).map(position_at);
                        let Some(previous) = previous else {
                            return Some(egui::Stroke::new(1.5_f32, beat_color(current.beat)));
                        };
                        if current.measure != previous.measure {
                            Some(egui::Stroke::new(
                                1.5_f32,
                                beat_color(current.beat).gamma_multiply(0.65),
                            ))
                        } else if current.beat != previous.beat {
                            Some(egui::Stroke::new(
                                0.7_f32,
                                beat_color(current.beat).gamma_multiply(0.65),
                            ))
                        } else {
                            None
                        }
                    });
                    let mbt = position
                        .as_ref()
                        .map_or_else(|| "—".into(), |p| columns.position(p));
                    let pitches: Vec<_> = if app.note_mode {
                        notes.keyboard_notes[index]
                            .iter()
                            .map(|&source| {
                                (
                                    events[source].data[0],
                                    active && app.selected_event == Some(source),
                                )
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };
                    let mut cells = if app.note_mode {
                        vec![mbt]
                    } else {
                        vec![
                            format!(
                                "{:>width$}",
                                index + 1,
                                width = events.len().to_string().len()
                            ),
                            format!("{:>width$}", event.delta, width = columns.delta),
                            format!("{:>width$}", event.tick, width = columns.absolute),
                            mbt,
                            raw_preview(event),
                        ]
                    };
                    // The note view's cells, also beside the raw bytes.
                    let content = cells.len();
                    let key = keys.and_then(|keys| keys.at(event.tick));
                    let spans = content::append(&mut cells, event, notes, index, key);
                    if !pitches.is_empty() {
                        cells.resize(4, String::new());
                    }
                    let refs = cells.iter().map(String::as_str).collect::<Vec<_>>();
                    let response = row(
                        ui,
                        &refs,
                        RowStyle {
                            note_mode: app.note_mode,
                            selected: active && app.cursor_row == Some(visible_index)
                                || active && app.selected_event == Some(index),
                            playing: marker == Some(visible_index),
                            preparing: preparing == Some(visible_index),
                            boundary,
                            beat_head,
                            position: position.as_ref(),
                            raw_event: if app.note_mode { None } else { Some(event) },
                            pitches: &pitches,
                            key,
                            pitch_range: columns.pitch_range,
                            widths: &widths,
                            caution: notes.overlaps[index].then_some(content + 1),
                            spans: Some((content, spans)),
                        },
                    );
                    let editable = app.note_mode && note_on(event);
                    if let Some(column) = response.clicked {
                        selected = Some(visible_index);
                        if cat_smf_editor::edit::trill::owned(event) {
                            trill_click = Some(index);
                        }
                        if editable && let Some(field) = Field::at(column) {
                            begin_edit = Some((
                                field,
                                index,
                                response.cells[column],
                                refs[column].to_owned(),
                            ));
                        }
                    } else if let Some(field) = app.note_edit_request
                        && editable
                        && active
                        && app.selected_event == Some(index)
                    {
                        let column = field.column();
                        begin_edit = Some((
                            field,
                            index,
                            response.cells[column],
                            refs[column].to_owned(),
                        ));
                    }
                }
            });
            super::event_selection::paint(selection.clone(), ui, &output, stride);
            (repeat, selecting_repeat) = super::event_repeat::rows(
                ui,
                &output,
                super::event_repeat::Rows {
                    loaded,
                    track,
                    note_mode: app.note_mode,
                    generation: app.generation,
                    range: app.playback.repeat,
                    stride,
                    count: row_count,
                },
            );
            if row_count > 0 {
                let first = (output.state.offset.y / stride) as usize;
                let last = ((output.state.offset.y + output.inner_rect.height()) / stride) as usize;
                let tick = |row: usize| {
                    crate::cursor::split::row_tick_loaded(loaded, app.note_mode, track, row)
                };
                let visible = (
                    tick(first.min(row_count - 1)),
                    if last < row_count {
                        tick(last)
                    } else {
                        events.last().map_or(0, |e| e.tick)
                    },
                );
                ui.ctx().data_mut(|d| {
                    d.insert_temp(
                        super::track_map::visible_id(app.generation, track, app.note_mode),
                        visible,
                    )
                });
                if active {
                    app.track_view.visible_ticks = Some(visible);
                }
            }
            let interacting = output
                .inner_rect
                .contains(ui.ctx().pointer_hover_pos().unwrap_or_default())
                && input_active;
            if row_count > 0
                && (app.track_view.split.is_none()
                    || interacting
                    || active && (app.reset_scroll || focus.is_some()))
            {
                // The shared time is the row under the viewport center.
                let middle = (output.state.offset.y + viewport * 0.5) / stride;
                let row = (middle.floor() as usize).min(row_count - 1);
                app.track_view.scroll_tick =
                    crate::cursor::split::row_tick_loaded(loaded, app.note_mode, track, row);
                app.track_view.scroll_fraction = middle.fract();
                app.track_view.scroll_source = Some((track, row));
            }
        });
    if app.track_view.split.is_none()
        || horizontal
            .inner_rect
            .contains(ui.ctx().pointer_hover_pos().unwrap_or_default())
            && horizontal_input
    {
        app.track_view.horizontal_offset = horizontal.state.offset.x;
    }
    if selecting_repeat {
        app.playback.follow = false;
    }
    if let Some(range) = repeat {
        crate::play::transport::repeat::start(app, range);
    }
    if !active && (patch || effect.is_some()) {
        crate::cursor::split::focus(app, 1 - app.track_view.active);
    }
    if patch {
        crate::view::layers::open(app);
    }
    *replace_effect = replace_effect.or(effect);
    if let Some((field, event, rect, text)) = begin_edit {
        crate::view::note_edit::begin(app, field, track, event, rect, &text);
    }
    if let Some(event) = trill_click
        && let Err(error) = crate::view::trill_edit::begin(app, track, event)
    {
        app.load_error = Some(error);
    }
    if let Some(index) = selected.filter(|_| trill_click.is_none()) {
        if !active {
            crate::cursor::split::focus(app, 1 - app.track_view.active);
        }
        crate::cursor::moves::select(app, index);
        if let Some(source) = app.selected_event {
            let event = &app.loaded.as_ref().unwrap().music.tracks[track].events[source];
            if event.status & 0xf0 == 0xc0 {
                *browse_patch = true;
            }
        }
    }
}

#[cfg(test)]
mod tests;
