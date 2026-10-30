use eframe::egui::{self, Pos2, Rect};

// Fast native input may deliver an entire gesture before egui's next frame.
// Its pointer state has already released the button and has no drag response.
pub(super) fn completed(ui: &egui::Ui, rect: Rect) -> Option<(Pos2, Pos2)> {
    completed_by(ui, rect, egui::PointerButton::Primary)
}

pub(super) fn completed_by(
    ui: &egui::Ui,
    rect: Rect,
    button: egui::PointerButton,
) -> Option<(Pos2, Pos2)> {
    let pair = ui.input(|i| {
        let mut start = None;
        let mut pair = None;
        let mut presses = 0;
        for event in &i.events {
            if let egui::Event::PointerButton {
                pos,
                button: event_button,
                pressed,
                ..
            } = event
                && *event_button == button
            {
                if *pressed {
                    presses += 1;
                    start = Some(*pos);
                } else if let Some(start) = start.take() {
                    pair = Some((start, *pos));
                }
            }
        }
        (presses == 1 && !i.pointer.button_down(button))
            .then_some(pair)
            .flatten()
    })?;
    (pair.0.distance(pair.1) > 6.0
        && rect.intersect(ui.clip_rect()).contains(pair.0)
        && ui.ctx().layer_id_at(pair.0) == Some(ui.layer_id()))
    .then_some(pair)
}
