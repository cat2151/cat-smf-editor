use super::*;

#[derive(Clone, Copy)]
pub(super) struct Edge {
    slot: usize,
    left: f32,
    right: f32,
    origin: f32,
    scale: f32,
    adjust_left: bool,
}

/// The chosen end is pinned at press time, so crossing the center cannot switch ends.
pub(super) fn adjust_edge(
    ui: &egui::Ui,
    rect: egui::Rect,
    scale: f32,
    response: &egui::Response,
    state: &mut State,
    slot: usize,
) -> bool {
    if ui.input(|i| i.pointer.secondary_pressed())
        && response.is_pointer_button_down_on()
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
    {
        let left = (ui.clip_rect().left() - rect.left()) / scale;
        let right = (ui.clip_rect().right() - rect.left()) / scale;
        let tick = ((pos.x - rect.left()) / scale).max(0.0);
        state.edge_drag = Some(Edge {
            slot,
            left,
            right,
            origin: rect.left(),
            scale,
            adjust_left: (tick - left).abs() <= (tick - right).abs(),
        });
    }
    let Some(drag) = state.edge_drag.filter(|d| d.slot == slot) else {
        return false;
    };
    let mut navigated = false;
    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        let tick = ((pos.x - drag.origin) / drag.scale).max(0.0);
        let (left, right) = if drag.adjust_left {
            (tick.min(drag.right - 1.0).max(0.0), drag.right)
        } else {
            (drag.left, tick.max(drag.left + 1.0))
        };
        let selected = egui::Rect::from_x_y_ranges(
            drag.origin + left * drag.scale..=drag.origin + right * drag.scale,
            rect.y_range(),
        );
        ui.painter().rect_filled(
            selected,
            0.0,
            Color32::from_rgb(0x66, 0xd9, 0xef).gamma_multiply(0.25),
        );
        if ui.input(|i| i.pointer.secondary_released()) {
            state.viewport.fit_time_range(
                left * drag.scale,
                right * drag.scale,
                crate::view::timeline_zoom::FIT,
            );
            navigated = true;
        }
    }
    if !ui.input(|i| i.pointer.secondary_down()) {
        state.edge_drag = None;
    }
    navigated
}
