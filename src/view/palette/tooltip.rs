use crate::view::commands::Command;
use eframe::egui;

pub(super) fn show(response: &egui::Response, command: &Command, selected: bool, hover: bool) {
    #[cfg(test)]
    response
        .ctx
        .data_mut(|d| d.insert_temp(egui::Id::new(("palette-row", &command.id)), response.rect));
    let description = command.description();
    if description.is_empty() {
        return;
    }
    let contents = |ui: &mut egui::Ui| {
        ui.set_max_width(420.0);
        ui.label(description);
    };
    if selected {
        response.show_tooltip_ui(contents);
    } else if hover {
        response.clone().on_hover_ui(contents);
    }
}
