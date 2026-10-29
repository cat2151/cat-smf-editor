//! Full-row selection with a keyboard shortcut beside the command label.
use super::commands::Command;
use eframe::egui::{self, Color32, FontId};

const GAP: f32 = 8.0;
const KEY_PADDING: egui::Vec2 = egui::vec2(6.0, 3.0);

fn key_font(style: &egui::Style) -> FontId {
    FontId::monospace(egui::TextStyle::Button.resolve(style).size)
}

pub(crate) fn width(ctx: &egui::Context, command: &Command) -> f32 {
    let style = ctx.style();
    ctx.fonts(|fonts| {
        let label = fonts.layout_no_wrap(
            command.label(),
            egui::TextStyle::Button.resolve(&style),
            Color32::WHITE,
        );
        let shortcut = command.key_label();
        label.size().x
            + if shortcut.is_empty() {
                0.0
            } else {
                GAP + fonts
                    .layout_no_wrap(shortcut, key_font(&style), Color32::WHITE)
                    .size()
                    .x
                    + 2.0 * KEY_PADDING.x
            }
    })
}

pub(crate) fn show(ui: &mut egui::Ui, command: &Command, selected: bool) -> egui::Response {
    action(ui, command.label(), command.key_label(), selected, false)
}

pub(crate) fn action(
    ui: &mut egui::Ui,
    label: String,
    shortcut: String,
    selected: bool,
    button: bool,
) -> egui::Response {
    let key = ui.painter().layout_no_wrap(
        shortcut.clone(),
        key_font(ui.style()),
        ui.visuals().text_color(),
    );
    let padding = ui.spacing().button_padding;
    let height = ui
        .spacing()
        .interact_size
        .y
        .max(key.size().y + 2.0 * (KEY_PADDING.y + padding.y));
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            format!("{label} {shortcut}"),
        )
    });
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        if button || selected || response.hovered() || response.has_focus() {
            ui.painter().rect(
                rect,
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }
        let content = rect.shrink2(padding);
        let text = ui.painter().layout_no_wrap(
            label,
            egui::TextStyle::Button.resolve(ui.style()),
            visuals.text_color(),
        );
        let mut label_rect = content;
        if !shortcut.is_empty() {
            let key_size = key.size() + 2.0 * KEY_PADDING;
            let key_left = (content.left() + text.size().x + GAP).min(content.right() - key_size.x);
            let key_rect = egui::Rect::from_min_size(
                egui::pos2(key_left, content.center().y - key_size.y / 2.0),
                key_size,
            );
            label_rect.max.x = (key_rect.left() - GAP).max(content.left());
            let painter = ui
                .painter()
                .with_clip_rect(content.intersect(ui.clip_rect()));
            let key_visuals = &ui.visuals().widgets.inactive;
            painter.rect_filled(
                key_rect.translate(egui::vec2(0.0, 1.0)),
                3.0,
                key_visuals.bg_stroke.color,
            );
            painter.rect(
                key_rect,
                3.0,
                ui.visuals().extreme_bg_color,
                key_visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
            painter.galley(key_rect.min + KEY_PADDING, key, ui.visuals().text_color());
        }
        ui.painter()
            .with_clip_rect(label_rect.intersect(ui.clip_rect()))
            .galley(
                egui::pos2(
                    label_rect.left(),
                    label_rect.center().y - text.size().y / 2.0,
                ),
                text,
                visuals.text_color(),
            );
    }
    response
}
