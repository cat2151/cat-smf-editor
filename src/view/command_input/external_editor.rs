use crate::app::App;
use eframe::egui::{self, Key, Modifiers};

pub(super) struct State {
    query: String,
    composing: bool,
    error: Option<String>,
}
impl State {
    pub(super) fn new(app: &App) -> Self {
        Self {
            query: app
                .external_editor
                .executable
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            composing: false,
            error: None,
        }
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
        match app.external_editor.save(&state.query) {
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
    crate::view::presentation::modal(ctx, egui::Id::new("external-editor-input")).show(ctx, |ui| {
        ui.heading("外部テキストエディタ");
        let id = egui::Id::new("external-editor-path");
        ctx.memory_mut(|m| m.request_focus(id));
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .id(id)
                .desired_width(600.0),
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
