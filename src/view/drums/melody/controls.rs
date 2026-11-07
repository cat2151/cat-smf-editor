use super::widgets::{CELL, HEADER};
use super::*;
use crate::view::timeline_zoom;
use egui::Color32;

const MIN_ZOOM: f32 = 0.5;

/// The vertical zoom bar left of the grid, drawn like the timelines' bars; returns the new
/// zoom. A click zooms in and a right click out around the clicked height, the wheel around
/// the pointer, and a drag makes the dragged span fill the view. The header row keeps its height.
pub(super) fn zoom_bar(ui: &mut egui::Ui, melody: &mut State, from: f32) -> Option<f32> {
    let (zone, response) = ui.allocate_exact_size(
        egui::vec2(timeline_zoom::BAR, ui.available_height()),
        egui::Sense::click_and_drag(),
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("melody-zoom-bar"), zone));
    let hovered = response.hovered();
    ui.painter().rect_filled(
        zone.shrink(2.0),
        2.0,
        if hovered {
            timeline_zoom::ACTIVE
        } else {
            Color32::from_gray(75)
        },
    );
    ui.painter().text(
        zone.center(),
        egui::Align2::CENTER_CENTER,
        "zoom",
        egui::FontId::monospace(9.0),
        if hovered {
            Color32::BLACK
        } else {
            Color32::from_gray(220)
        },
    );
    let local = |p: egui::Pos2| (p.y - zone.top()).clamp(0.0, zone.height());
    let pointer = response.interact_pointer_pos().map(local);
    if response.drag_started_by(egui::PointerButton::Primary) {
        melody.drag = ui.input(|i| i.pointer.press_origin()).map(local);
    }
    let mut fit = None;
    if let (Some(start), Some(end)) = (melody.drag, pointer) {
        let span = egui::Rect::from_x_y_ranges(
            zone.x_range(),
            zone.top() + start.min(end)..=zone.top() + start.max(end),
        );
        ui.painter()
            .rect_filled(span, 0.0, timeline_zoom::ACTIVE.gamma_multiply(0.5));
        if response.drag_stopped() && (start - end).abs() > 2.0 {
            fit = Some((start, end));
        }
    }
    if response.drag_stopped() {
        melody.drag = None;
    }
    let wheel = if hovered {
        ui.input_mut(|i| std::mem::take(&mut i.smooth_scroll_delta).y)
    } else {
        0.0
    };
    // (factor, the height in the view to zoom around, where that content lands)
    let (factor, anchor, target) = match (fit, pointer) {
        (Some((start, end)), _) => {
            let middle = zone.height() / 2.0;
            (
                zone.height() / (start - end).abs(),
                (start + end) / 2.0,
                middle,
            )
        }
        (None, Some(y)) if response.clicked() => (timeline_zoom::CLICK, y, y),
        (None, Some(y)) if response.secondary_clicked() => (1.0 / timeline_zoom::CLICK, y, y),
        _ if wheel != 0.0 => {
            let y = response.hover_pos().map_or(zone.height() / 2.0, local);
            (timeline_zoom::wheel(wheel), y, y)
        }
        _ => return None,
    };
    let to = (from * factor).clamp(MIN_ZOOM, (zone.height() / CELL.y).max(MIN_ZOOM));
    let below_header = melody.pending.unwrap_or(melody.scroll) + anchor - HEADER;
    melody.pending = Some((below_header * to / from + HEADER - target).max(0.0));
    Some(to)
}

/// One label per row set, the shown one selected; a click picks that set.
pub(super) fn row_sets(ui: &mut egui::Ui, shown: harmony::Set) -> Option<harmony::Set> {
    let mut picked = None;
    for set in harmony::Set::ALL {
        let name = match set {
            harmony::Set::Chromatic => "Chromatic",
            harmony::Set::Scale => "Scale",
            harmony::Set::Penta => "Penta",
            harmony::Set::Chord => "Chord",
        };
        if ui
            .selectable_label(set == shown, name)
            .on_hover_text("/")
            .clicked()
        {
            picked = Some(set);
        }
    }
    picked
}
