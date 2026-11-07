//! Popup callbacks register the viewport actually displaying their content.
use eframe::egui;

pub(crate) fn context_menu(
    response: &egui::Response,
    body: impl FnOnce(&mut egui::Ui),
) -> Option<egui::InnerResponse<()>> {
    response.context_menu(|ui| {
        super::claim(ui.ctx(), true);
        body(ui);
    })
}

pub(crate) fn menu_button<R>(
    ui: &mut egui::Ui,
    title: impl Into<egui::WidgetText>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    ui.menu_button(title, |ui| {
        super::claim(ui.ctx(), true);
        body(ui)
    })
}

pub(crate) fn popup_below_widget<R>(
    ui: &egui::Ui,
    id: egui::Id,
    response: &egui::Response,
    behavior: egui::PopupCloseBehavior,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    egui::popup_below_widget(ui, id, response, behavior, |ui| {
        super::claim(ui.ctx(), true);
        body(ui)
    })
}
