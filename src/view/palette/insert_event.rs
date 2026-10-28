//! Event insertion stays inside the palette's suspended input owner.
use crate::app::App;
use cat_smf_editor::edit::meta_input::Kind;
use eframe::egui::{self, Key, Modifiers};

#[derive(Default)]
pub(super) struct State {
    kind: Option<Kind>,
    query: String,
    search: crate::view::commands::SearchQuery,
    selected: usize,
    composing: bool,
    error: Option<String>,
}

impl State {
    /// The input state opened by a palette command id, if the id belongs to event insertion.
    pub(super) fn for_command(id: &str) -> Option<Self> {
        let kind = match id {
            "insert-event" => None,
            "insert-tempo" => Some(Kind::Tempo),
            "insert-key-signature" => Some(Kind::KeySignature),
            _ => return None,
        };
        Some(Self {
            kind,
            ..Self::default()
        })
    }
}

fn apply(app: &mut App, kind: Kind, text: &str) -> Result<(), String> {
    let tick = crate::play::playback::location(app).0;
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
    let mut kinds: Vec<_> = [Kind::Tempo, Kind::KeySignature]
        .into_iter()
        .filter(|k| state.search.matches_text(k.label()))
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
            match apply(app, kind, &state.query) {
                Ok(()) => return true,
                Err(error) => state.error = Some(error),
            }
        } else if let Some(&kind) = kinds.get(state.selected) {
            state.kind = Some(kind);
            state.query.clear();
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
                    .hint_text(state.kind.map_or("Event type", Kind::placeholder))
                    .desired_width(f32::INFINITY),
            );
            response.request_focus();
            if response.changed() {
                state.error = None;
                state.selected = 0;
                state.search.update(&state.query);
                // Rebuild to include entries when the filter is shortened.
                kinds = [Kind::Tempo, Kind::KeySignature]
                    .into_iter()
                    .filter(|kind| state.search.matches_text(kind.label()))
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
                        state.query.clear();
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
mod tests {
    use super::*;
    use crate::view::palette::tests::{app, frame, open, press, query};

    #[test]
    fn inserts_immediately_before_current_event_in_notes_and_raw() {
        for note_mode in [true, false] {
            for source in 0..3 {
                let mut a = app();
                let mut music = a.loaded.as_ref().unwrap().music.clone();
                music.tracks[2]
                    .events
                    .insert(1, Kind::Tempo.event(0, "90").unwrap());
                music.tracks[2]
                    .events
                    .insert(2, Kind::KeySignature.event(0, "C major").unwrap());
                a.apply_music(music.clone()).unwrap();
                a.track = 2;
                a.note_mode = note_mode;
                let row = if note_mode {
                    a.loaded.as_ref().unwrap().display_rows[2]
                        .event_row(source)
                        .unwrap()
                } else {
                    source
                };
                crate::cursor::moves::place(&mut a, row);

                apply(&mut a, Kind::Tempo, "120").unwrap();

                let events = &a.loaded.as_ref().unwrap().music.tracks[2].events;
                assert_eq!(events[source].meta_type, 0x51);
                assert_eq!(events[source].data, [7, 161, 32]);
                let mut original = events.clone();
                original.remove(source);
                assert_eq!(original.len(), music.tracks[2].events.len());
                for (actual, expected) in original.iter().zip(&music.tracks[2].events) {
                    assert_eq!(
                        (
                            actual.tick,
                            actual.delta,
                            actual.status,
                            actual.meta_type,
                            &actual.data
                        ),
                        (
                            expected.tick,
                            expected.delta,
                            expected.status,
                            expected.meta_type,
                            &expected.data
                        )
                    );
                }
                assert_eq!(a.selected_event, Some(source));
                assert_eq!(crate::play::playback::location(&a).0, 0);
                a.undo_music().unwrap();
                assert_eq!(
                    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
                    cat_smf_editor::smf_io::encode(&music).unwrap()
                );
            }
        }
    }

    #[test]
    fn exact_tick_inserts_before_same_tick_events_without_using_stale_selection() {
        let mut a = app();
        a.note_mode = false;
        crate::cursor::moves::place(&mut a, 0);
        a.track_view.cursor_tick = Some(480);
        apply(&mut a, Kind::Tempo, "120").unwrap();
        let events = &a.loaded.as_ref().unwrap().music.tracks[0].events;
        assert_eq!(events[1].meta_type, 0x51);
        assert_eq!(events[1].tick, 480);
        assert_eq!(events[1].delta, 480);
        assert_eq!(events[2].status, 0x80);
        assert_eq!(events[2].tick, 480);
        assert_eq!(events[2].delta, 0);
        assert_eq!(a.selected_event, Some(1));
    }
    #[test]
    fn invalid_tempo_and_escape_leave_song_and_undo_unchanged() {
        let mut a = app();
        let c = egui::Context::default();
        let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
        open(&mut a, &c);
        query(&mut a, &c, "insert event");
        press(&mut a, &c, Key::Enter);
        frame(&mut a, &c, vec![]);
        press(&mut a, &c, Key::Enter);
        frame(&mut a, &c, vec![egui::Event::Text("NaN".into())]);
        press(&mut a, &c, Key::Enter);
        let state = a.palette.insert_event.as_ref().unwrap();
        assert_eq!(state.query, "NaN");
        assert!(state.error.is_some());
        assert!(!a.undo.available());
        press(&mut a, &c, Key::Escape);
        assert!(!a.palette.is_open());
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
    }
    #[test]
    fn tempo_targets_cursor_track_tick_and_preserves_end_of_track() {
        let mut a = app();
        a.track = 2;
        a.track_view.cursor_tick = Some(960);
        a.loaded.as_mut().unwrap().music.tracks[2]
            .events
            .push(cat_smf_editor::model::Event {
                tick: 480,
                delta: 0,
                status: 0xff,
                meta_type: 0x2f,
                data: vec![],
            });
        apply(&mut a, Kind::Tempo, "123.45").unwrap();
        let music = &a.loaded.as_ref().unwrap().music;
        assert!(!music.tracks[0].events.iter().any(|e| e.meta_type == 0x51));
        let events = &music.tracks[2].events;
        assert_eq!(events[2].tick, 960);
        assert_eq!(events[2].delta, 480);
        assert_eq!(events[2].data, [7, 106, 139]);
        assert_eq!(events.last().unwrap().meta_type, 0x2f);
        assert_eq!(events.last().unwrap().tick, 960);
        let bytes = cat_smf_editor::smf_io::encode(music).unwrap();
        assert!(!bytes.is_empty());
    }
    /// Opens a direct insertion command from the palette and types `value` into its input.
    fn direct(a: &mut App, c: &egui::Context, command: &str, value: &str) {
        open(a, c);
        query(a, c, command);
        press(a, c, Key::Enter);
        // The modal takes focus on the frame after the palette chose the command.
        frame(a, c, vec![]);
        assert!(a.palette.insert_event.as_ref().unwrap().kind.is_some());
        frame(a, c, vec![egui::Event::Text(value.into())]);
        press(a, c, Key::Enter);
    }

    #[test]
    fn direct_commands_skip_the_kind_list_and_insert_undoably() {
        for (command, value, meta_type, data) in [
            ("insert tempo", "140", 0x51, vec![6, 138, 27]),
            ("insert key signature", "D minor", 0x59, vec![255, 1]),
        ] {
            let mut a = app();
            let c = egui::Context::default();
            let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
            direct(&mut a, &c, command, value);
            assert!(
                !a.palette.is_open(),
                "{command}: state={:?}",
                a.palette
                    .insert_event
                    .as_ref()
                    .map(|s| (s.kind, &s.query, &s.error))
            );
            let events = &a.loaded.as_ref().unwrap().music.tracks[a.track].events;
            assert_eq!(events[0].meta_type, meta_type, "{command}");
            assert_eq!(events[0].data, data, "{command}");
            assert!(a.undo.available());
            a.undo_music().unwrap();
            assert_eq!(
                cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
                before
            );
        }
    }

    #[test]
    fn direct_tempo_rejects_invalid_value_and_escape_closes() {
        let mut a = app();
        let c = egui::Context::default();
        let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
        direct(&mut a, &c, "insert tempo", "NaN");
        let state = a.palette.insert_event.as_ref().unwrap();
        assert_eq!(state.kind, Some(Kind::Tempo));
        assert_eq!(state.query, "NaN");
        assert!(state.error.is_some());
        assert!(!a.undo.available());
        press(&mut a, &c, Key::Escape);
        assert!(!a.palette.is_open());
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
    }

    #[test]
    fn keyboard_insertion_is_atomic_undoable_and_suspends_shortcuts() {
        let mut a = app();
        let c = egui::Context::default();
        let before = a.loaded.as_ref().unwrap().music.clone();
        open(&mut a, &c);
        query(&mut a, &c, "insert event");
        press(&mut a, &c, Key::Enter);
        frame(&mut a, &c, vec![]);
        press(&mut a, &c, Key::ArrowDown);
        press(&mut a, &c, Key::Enter);
        frame(&mut a, &c, vec![egui::Event::Text("D minor".into())]);
        press(&mut a, &c, Key::Enter);
        assert!(
            !a.palette.is_open(),
            "state={:?}",
            a.palette
                .insert_event
                .as_ref()
                .map(|s| (s.kind, &s.query, &s.error))
        );
        assert!(a.loaded.as_ref().unwrap().music.tracks[0]
            .events
            .iter()
            .any(|e| e.meta_type == 0x59 && e.data == [255, 1]));
        assert!(!a.playback.open);
        assert!(a.undo.available());
        let events = &a.loaded.as_ref().unwrap().music.tracks[0].events;
        assert_eq!(events[0].meta_type, 0x59);
        assert_eq!(events[1].status, 0x90);
        assert_eq!(a.selected_event, Some(0));
        a.undo_music().unwrap();
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            cat_smf_editor::smf_io::encode(&before).unwrap()
        );
    }
}
