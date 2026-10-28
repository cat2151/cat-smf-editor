//! Enter during composition belongs to the IME, never an editor confirmation.
use eframe::egui::{self, Event, ImeEvent, Key};
pub fn keep_ime(ctx: &egui::Context) {
    let id = egui::Id::new(("note-input-ime", ctx.viewport_id()));
    let mut composing = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let mut ime = composing;
    ctx.input_mut(|i| {
        for event in &i.events {
            if let Event::Ime(e) = event {
                ime = true;
                match e {
                    ImeEvent::Preedit(s) => composing = !s.is_empty(),
                    ImeEvent::Commit(_) | ImeEvent::Disabled => composing = false,
                    _ => {}
                }
            }
        }
        if ime {
            i.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: Key::Enter | Key::Escape,
                        ..
                    }
                )
            });
        }
    });
    ctx.data_mut(|d| d.insert_temp(id, composing));
}
