use crate::app::App;
use eframe::egui;

mod content;
mod event_list;
mod header;

pub(crate) use header::close_warnings_on_escape;

pub fn show(
    app: &mut App,
    ui: &mut egui::Ui,
    track: usize,
    active: bool,
    browse_patch: &mut bool,
    replace_effect: &mut Option<usize>,
) {
    let name = super::patterns::presentation::name(app, track);
    super::patterns::presentation::frame(app, track).show(ui, |ui| {
        if let Some(name) = &name {
            super::patterns::presentation::label(ui, name);
        }
        event_list::body(app, ui, track, active, browse_patch, replace_effect);
    });
}

#[cfg(test)]
mod tests;
