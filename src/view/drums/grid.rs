use super::{commands::Op, *};
use egui::Rect;

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut egui::Ui,
    music: &cat_smf_editor::model::Music,
    track: usize,
    patch: &Patch,
    state: &mut State,
    measure: &drums::Measure,
    mut surface: super::surface::Surface,
    action: &mut Option<Op>,
) {
    let cursor = surface.cursor;
    let width = ui.available_width();
    let lane = (width * 0.14).clamp(56.0, 94.0);
    let instrument = (width * 0.23).clamp(64.0, 190.0);
    let step_x = lane + instrument + 12.0;
    let pitch = ((width - step_x) / measure.steps.len().max(1) as f32).max(0.0);
    let gap = (pitch * 0.15).min(4.0);
    let (area, _) = ui.allocate_exact_size(egui::vec2(width, 312.0), egui::Sense::hover());
    let rect = |x, y, w, h| Rect::from_min_size(area.min + egui::vec2(x, y), egui::vec2(w, h));
    ui.put(
        rect(lane + 4.0, 0.0, instrument, 24.0),
        egui::Label::new("Instrument").truncate(),
    );
    for step in 0..measure.steps.len() {
        let x = step_x + step as f32 * pitch;
        surface::column(
            ui,
            step,
            rect(x, 0.0, pitch - gap, 24.0),
            pitch,
            area.left() + x - gap * 0.5,
            area,
        );
    }
    #[cfg(test)]
    let mut cells = Vec::new();
    for (visual, row) in (0..4).rev().enumerate() {
        let y = 24.0 + visual as f32 * 72.0;
        let layer = patch.drums.as_ref().and_then(|s| {
            patch
                .layers
                .iter()
                .find(|l| l.settings.id == s.rows[row].layer)
        });
        let muted = layer.is_some_and(|l| l.settings.mute);
        let text = surface::lane_text(drums::NAMES[row], muted);
        let response = ui
            .put(
                rect(0.0, y, lane, 34.0),
                egui::Button::new(text).frame(false).truncate(),
            )
            .on_hover_text("Mute · M");
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(("drum-mute", row)), response.rect));
        if response.clicked() {
            state.row = row;
            *action = Some(Op::Mute(row));
        }
        let name = layer.map_or("Surge XT", |l| l.label());
        for field in 0..measure.steps.len() + 2 {
            if field == 1 {
                #[cfg(test)]
                cells.push(Rect::NOTHING);
                continue;
            }
            let (label, bounds) = match field {
                0 => (
                    name.rsplit(['/', '\\']).next().unwrap_or(name).to_owned(),
                    rect(lane + 4.0, y, instrument, 34.0),
                ),
                _ => (
                    String::new(),
                    rect(step_x + (field - 2) as f32 * pitch, y, pitch - gap, 34.0),
                ),
            };
            let active = field >= 2 && measure.active(&music.tracks[track], row, field - 2);
            let fill = surface::step_fill(ui, active, muted && field >= 2);
            let mut button = egui::Button::new(label).truncate().fill(fill);
            if state.row == row && state.field == field {
                button = button.stroke(cursor);
            }
            let response = ui.put(bounds, button);
            surface.selection(ui, response.rect, state.row == row && state.field == field);
            if field >= 2 {
                surface.cell(response.rect, field - 2);
            }
            #[cfg(test)]
            cells.push(response.rect);
            if response.clicked() {
                state.row = row;
                state.field = field;
                *action = Some(Op::Activate);
            }
        }
        super::values::controls(
            ui,
            rect(
                4.0 + lane.min((width - 360.0).max(0.0)),
                y + 36.0,
                width - 4.0 - lane.min((width - 360.0).max(0.0)),
                30.0,
            ),
            patch,
            state,
            row,
            measure.steps.len() + 2,
            cursor,
            action,
        );
        #[cfg(test)]
        {
            cells[visual * (measure.steps.len() + 2) + 1] = ui.ctx().data(|d| {
                d.get_temp::<Rect>(egui::Id::new(("drum-note", row)))
                    .unwrap()
            });
        }
    }
    surface.finish(ui);
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new("drum-grid-bounds"),
            (area, ui.clip_rect(), cells),
        )
    });
}
