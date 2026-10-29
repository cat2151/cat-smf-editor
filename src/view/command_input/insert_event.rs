//! Meta-event arguments and their shared insertion operation.
use crate::app::App;
use cat_smf_editor::edit::meta_input::Kind;
use eframe::egui::{self, Key, Modifiers};

#[derive(Default)]
pub(super) struct State {
    pub(super) tick: u64,
    pub(super) kind: Option<Kind>,
    pub(super) edit: Option<usize>,
    pub(super) move_event: bool,
    pub(super) query: String,
    pub(super) pending_text: Option<String>,
    pub(super) search: crate::view::commands::SearchQuery,
    pub(super) selected: usize,
    pub(super) composing: bool,
    pub(super) error: Option<String>,
}

pub(super) fn apply(app: &mut App, kind: Kind, text: &str) -> Result<(), String> {
    if kind == Kind::Chord
        && cat_smf_editor::score::chord_guide::track(
            &app.loaded.as_ref().ok_or("No SMF is loaded")?.music,
        ) != Some(app.track)
    {
        return Err("Select the chord guide track".into());
    }
    apply_at(app, kind, text, crate::play::playback::location(app).0)
}

fn apply_at(app: &mut App, kind: Kind, text: &str, tick: u64) -> Result<(), String> {
    let before = app.loaded.as_ref().ok_or("No SMF is loaded")?.music.clone();
    let mut music = before.clone();
    let track = music.tracks.get_mut(app.track).ok_or("Invalid track")?;
    let source = if app.track_view.cursor_tick.is_some() || app.cursor_position.is_some() {
        None
    } else if app.note_mode {
        app.cursor_row.map_or(app.selected_event, |row| {
            match app.loaded.as_ref().unwrap().display_rows[app.track].get(row) {
                cat_smf_editor::score::display_rows::DisplayRow::Event(index) => Some(index),
                cat_smf_editor::score::display_rows::DisplayRow::Beat(_) => None,
            }
        })
    } else {
        app.cursor_row.or(app.selected_event)
    };
    let end = track
        .events
        .pop_if(|event| event.status == 0xff && event.meta_type == 0x2f);
    let at = source
        .filter(|&index| index <= track.events.len())
        .unwrap_or_else(|| track.events.partition_point(|event| event.tick < tick));
    track.events.insert(at, kind.event(tick, text)?);
    if let Some(mut end) = end {
        end.tick = end.tick.max(tick);
        track.events.push(end);
    }
    cat_smf_editor::edit::refresh_deltas(track)?;
    app.apply_music(music)?;
    app.undo.push(before);
    app.selected_event = Some(at);
    crate::cursor::moves::sync_mode(app);
    app.reveal_cursor = true;
    Ok(())
}

pub(super) fn show(app: &mut App, ctx: &egui::Context, state: &mut State, fresh: bool) -> bool {
    let events = ctx.input(|i| i.events.clone());
    let ime = state.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    let text_input = events.iter().any(|e| {
        matches!(
            e,
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
        )
    });
    state.search.update(&state.query);
    let mut kinds: Vec<_> = [Kind::Tempo, Kind::KeySignature, Kind::Chord]
        .into_iter()
        .filter(|k| {
            (*k != Kind::Chord || super::guide_edit::on_guide(app))
                && state.search.matches_text(k.label())
        })
        .collect();
    let mut enter = false;
    let mut escape = false;
    if fresh {
        for event in &events {
            if let egui::Event::Ime(event) = event {
                match event {
                    egui::ImeEvent::Preedit(text) => state.composing = !text.is_empty(),
                    egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => state.composing = false,
                    _ => {}
                }
            }
            if !ime
                && let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } = event
            {
                if state.kind.is_none()
                    && let Some(movement) = crate::view::list_navigation::movement(*key, *modifiers)
                {
                    state.selected = movement.apply(state.selected, kinds.len());
                }
                if super::commands::matches(*modifiers, Modifiers::NONE) && !repeat {
                    enter |= *key == Key::Enter && !text_input;
                    escape |= *key == Key::Escape;
                }
            }
        }
    }
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            !matches!(
                e,
                egui::Event::Key {
                    key: Key::Enter | Key::Escape,
                    ..
                }
            ) || ime
        })
    });
    if !fresh {
        super::clear(ctx);
    }
    if escape {
        return true;
    }
    if enter {
        if let Some(kind) = state.kind {
            let result = if let Some(index) = state.edit {
                super::guide_edit::apply(app, index, kind, &state.query, state.move_event)
            } else {
                apply_at(app, kind, &state.query, state.tick)
            };
            match result {
                Ok(()) => return true,
                Err(error) => state.error = Some(error),
            }
        } else if let Some(&kind) = kinds.get(state.selected) {
            state.kind = Some(kind);
            state.query = state.pending_text.take().unwrap_or_default();
            state.error = None;
            super::clear(ctx);
        }
    }
    egui::Modal::new(egui::Id::new("insert-event"))
        .area(
            egui::Modal::default_area(egui::Id::new("insert-event"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 8.0]),
        )
        .show(ctx, |ui| {
            ui.set_width(320.0_f32.min((ctx.screen_rect().width() - 48.0).max(1.0)));
            let id = egui::Id::new(("insert-event-input", state.kind));
            ctx.memory_mut(|m| m.request_focus(id));
            let response = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .id(id)
                    .hint_text(if state.move_event {
                        "Tick"
                    } else {
                        state.kind.map_or("Event type", Kind::placeholder)
                    })
                    .desired_width(f32::INFINITY),
            );
            response.request_focus();
            if response.changed() {
                state.error = None;
                state.selected = 0;
                state.search.update(&state.query);
                // Rebuild to include entries when the filter is shortened.
                kinds = [Kind::Tempo, Kind::KeySignature, Kind::Chord]
                    .into_iter()
                    .filter(|kind| {
                        (*kind != Kind::Chord || super::guide_edit::on_guide(app))
                            && state.search.matches_text(kind.label())
                    })
                    .collect();
            }
            if state.kind.is_none() {
                state.selected = state.selected.min(kinds.len().saturating_sub(1));
                for (index, kind) in kinds.iter().enumerate() {
                    if ui
                        .selectable_label(index == state.selected, kind.label())
                        .clicked()
                    {
                        state.kind = Some(*kind);
                        state.query = state.pending_text.take().unwrap_or_default();
                        state.error = None;
                        ctx.request_repaint();
                    }
                }
            }
            if let Some(error) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
    false
}

#[cfg(test)]
mod tests;
