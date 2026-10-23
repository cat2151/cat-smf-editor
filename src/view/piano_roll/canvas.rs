use super::*;

#[derive(Default)]
pub(super) struct Action {
    pub edit: Option<(usize, hover::Part, i64)>,
    pub audition: Option<u64>,
    pub navigated: bool,
}

/// Edits only the pane that owns the drag, even when both panes show one track.
pub(super) fn show(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    layout: &Layout,
    state: &mut State,
    slot: usize,
) -> Action {
    let mut action = Action::default();
    // A fixed id keeps the drag alive when the error line above appears or goes.
    let response = ui.interact(
        rect,
        crate::view::palette::preview_id(ui.ctx(), egui::Id::new(("piano-roll-notes", slot))),
        egui::Sense::click_and_drag(),
    );
    let roll = |pos: egui::Pos2| (pos - rect.min).to_pos2();
    let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
    if primary_pressed && response.is_pointer_button_down_on() {
        state.error = None;
        state.drag_pane = Some(slot);
        state.drag = ui
            .input(|i| i.pointer.press_origin())
            .map(roll)
            .and_then(|pos| {
                hover::hovered(&layout.notes, pos)
                    .map(|(note, part)| drag::Drag::new(note, part, pos.x))
            });
        state.range_drag = if state.drag.is_none() {
            ui.input(|i| i.pointer.press_origin())
                .map(roll)
                .map(|pos| (slot, pos.x))
        } else {
            None
        };
    }
    let active_drag = state.drag.filter(|_| state.drag_pane == Some(slot));
    let moved = active_drag.map(|drag| {
        let shift = response
            .interact_pointer_pos()
            .or(response.hover_pos())
            .map_or(0, |pos| drag.shift(roll(pos).x, layout.scale));
        (drag, shift)
    });
    let released = state.drag_pane == Some(slot) && ui.input(|i| i.pointer.primary_released());
    if let Some((_, start)) = state.range_drag.filter(|(owner, _)| *owner == slot)
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
    {
        let end = pos.x.clamp(ui.clip_rect().left(), ui.clip_rect().right()) - rect.left();
        let selected = egui::Rect::from_x_y_ranges(
            rect.left() + start.min(end)..=rect.left() + start.max(end),
            rect.y_range(),
        );
        ui.painter().rect_filled(
            selected,
            0.0,
            Color32::from_rgb(0x66, 0xd9, 0xef).gamma_multiply(0.25),
        );
        if released && (start - end).abs() > 2.0 {
            state
                .viewport
                .fit_time_range(start, end, crate::view::timeline_zoom::FIT);
            action.navigated = true;
        }
    }
    if released {
        state.range_drag = None;
        state.drag = None;
        state.drag_pane = None;
        if let Some((drag, shift)) = moved.filter(|&(_, shift)| shift != 0) {
            let on = layout.notes[drag.note].0;
            action.edit = Some((on, drag.part, shift));
        }
    }
    let hover = match active_drag {
        Some(drag) => Some((drag.note, drag.part)),
        None => response
            .hover_pos()
            .and_then(|pos| hover::hovered(&layout.notes, roll(pos))),
    };
    if hover.is_some() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(
                hover::BLINK_PERIOD / 2.0,
            ));
    }
    let moved = moved.filter(|_| state.drag.is_some()).map(|(drag, shift)| {
        let note = layout.notes[drag.note].1;
        (drag.note, drag.moved(note, shift, layout.scale))
    });
    let lit = hover::lit(ui.input(|i| i.time));
    draw(ui.painter(), rect, layout, moved, hover.filter(|_| lit));
    action.navigated |= navigation::adjust_edge(ui, rect, layout.scale, &response, state, slot);
    if response.clicked()
        && action.edit.is_none()
        && !action.navigated
        && let Some(pos) = response.interact_pointer_pos()
    {
        action.audition = Some((roll(pos).x.max(0.0) / layout.scale).round() as u64);
    }
    action
}
