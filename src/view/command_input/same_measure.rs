use crate::app::App;
use cat_smf_editor::score::same_measure;
use eframe::egui::{self, Key, Modifiers};

pub(super) struct State {
    track: usize,
    destination: u64,
    range: Option<(u64, u64)>,
    query: String,
    composing: bool,
    error: Option<String>,
}
impl State {
    pub(super) fn new(app: &App, batch: bool) -> Result<Self, String> {
        let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
        let map = loaded
            .time
            .map(app.track)
            .ok_or("Same measure requires PPQN timing")?;
        Ok(Self {
            track: app.track,
            destination: u64::try_from(
                map.position(crate::play::playback::location(app).0).measure,
            )
            .map_err(|_| "Measure overflow")?,
            range: if batch {
                Some(crate::view::event_selection::measures(app).ok_or("No measure selection")?)
            } else {
                None
            },
            query: String::new(),
            composing: false,
            error: None,
        })
    }
    pub(super) fn set_number(&mut self, number: u64) {
        self.query = number.to_string();
    }
    pub(super) fn apply(&self, app: &mut App) -> Result<(), String> {
        let number = self
            .query
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|&n| n > 0)
            .ok_or("Enter a positive measure number")?;
        let (first, last, destination) = self
            .range
            .map_or((number, number, self.destination), |(a, b)| (a, b, number));
        app.edit_music(|music| same_measure::insert(music, self.track, first, last, destination))?;
        app.event_selection = Default::default();
        Ok(())
    }
}

pub(super) fn show(app: &mut App, ctx: &egui::Context, state: &mut State, fresh: bool) -> bool {
    let events = ctx.input(|i| i.events.clone());
    let ime = state.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    let text = events.iter().any(|e| {
        matches!(
            e,
            egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
        )
    });
    let mut enter = false;
    let mut cancel = false;
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
                    repeat: false,
                    modifiers,
                    ..
                } = event
                && super::commands::matches(*modifiers, Modifiers::NONE)
            {
                enter |= *key == Key::Enter && !text;
                cancel |= *key == Key::Escape;
            }
        }
    }
    if cancel {
        return true;
    }
    if enter {
        match state.apply(app) {
            Ok(()) => return true,
            Err(error) => state.error = Some(error),
        }
    }
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            ime || !matches!(
                e,
                egui::Event::Key {
                    key: Key::Enter | Key::Escape,
                    ..
                }
            )
        })
    });
    if !fresh {
        super::clear(ctx);
    }
    egui::Modal::new(egui::Id::new("same-measure-input")).show(ctx, |ui| {
        ui.heading(if state.range.is_some() {
            "Generate same measures"
        } else {
            "Insert same measure"
        });
        let id = egui::Id::new("same-measure-number");
        ctx.memory_mut(|m| m.request_focus(id));
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(id)
                .hint_text(if state.range.is_some() {
                    "Destination measure"
                } else {
                    "Source measure"
                }),
        );
        response.request_focus();
        if response.changed() {
            state.error = None;
        }
        if let Some(error) = &state.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    });
    false
}

#[cfg(test)]
mod tests;
