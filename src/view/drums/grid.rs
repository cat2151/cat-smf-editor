use super::{commands::Op, *};
use egui::{Color32, Rect, Stroke};

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut egui::Ui,
    music: &cat_smf_editor::model::Music,
    track: usize,
    patch: &Patch,
    state: &mut State,
    measure: &drums::Measure,
    beacon: bool,
    action: &mut Option<Op>,
) {
    let width = ui.available_width();
    let lane = (width * 0.14).clamp(56.0, 94.0);
    let instrument = (width * 0.23).clamp(64.0, 190.0);
    let note = (width * 0.065).clamp(28.0, 44.0);
    let step_x = lane + instrument + note + 16.0;
    let pitch = ((width - step_x) / measure.steps.len().max(1) as f32).max(0.0);
    let gap = (pitch * 0.15).min(4.0);
    let (area, _) = ui.allocate_exact_size(egui::vec2(width, 192.0), egui::Sense::hover());
    let rect = |x, y, w, h| Rect::from_min_size(area.min + egui::vec2(x, y), egui::vec2(w, h));
    ui.put(
        rect(lane + 4.0, 0.0, instrument, 24.0),
        egui::Label::new("Instrument").truncate(),
    );
    ui.put(
        rect(lane + instrument + 8.0, 0.0, note, 24.0),
        egui::Label::new("Note").truncate(),
    );
    for step in 0..measure.steps.len() {
        let x = step_x + step as f32 * pitch;
        if pitch >= 16.0 {
            ui.put(
                rect(x, 0.0, pitch - gap, 24.0),
                egui::Label::new((step + 1).to_string()).truncate(),
            );
        }
        if step % 4 == 0 {
            let x = area.left() + x - gap * 0.5;
            ui.painter().line_segment(
                [egui::pos2(x, area.top()), egui::pos2(x, area.bottom())],
                Stroke::new(2.0_f32, Color32::from_gray(150)),
            );
        }
    }
    let mut beat = Rect::NOTHING;
    #[cfg(test)]
    let mut cells = Vec::new();
    for (visual, row) in (0..4).rev().enumerate() {
        let y = 24.0 + visual as f32 * 42.0;
        ui.put(
            rect(0.0, y, lane, 34.0),
            egui::Label::new(drums::NAMES[row]).truncate(),
        );
        let layer = patch.drums.as_ref().and_then(|s| {
            patch
                .layers
                .iter()
                .find(|l| l.settings.id == s.rows[row].layer)
        });
        let name = layer.map_or("Surge XT", |l| l.label());
        let output = patch.drums.as_ref().map_or(60, |s| s.rows[row].output);
        for field in 0..measure.steps.len() + 2 {
            let (label, bounds) = match field {
                0 => (
                    name.rsplit(['/', '\\']).next().unwrap_or(name).to_owned(),
                    rect(lane + 4.0, y, instrument, 34.0),
                ),
                1 => (
                    output.to_string(),
                    rect(lane + instrument + 8.0, y, note, 34.0),
                ),
                _ => (
                    String::new(),
                    rect(step_x + (field - 2) as f32 * pitch, y, pitch - gap, 34.0),
                ),
            };
            let active = field >= 2 && measure.active(&music.tracks[track], row, field - 2);
            let fill = if active {
                Color32::from_rgb(166, 226, 46)
            } else {
                ui.visuals().widgets.inactive.bg_fill
            };
            let mut button = egui::Button::new(label).truncate().fill(fill);
            if state.row == row && state.field == field {
                button = button.stroke(Stroke::new(2.0_f32, super::super::ACTIVE_PANE));
            }
            let response = ui.put(bounds, button);
            if (2..6).contains(&field) {
                beat = beat.union(response.rect);
            }
            #[cfg(test)]
            cells.push(response.rect);
            if response.clicked() {
                state.row = row;
                state.field = field;
                *action = Some(Op::Activate);
            }
        }
    }
    // The first beat of an empty pattern is where input starts.
    if beacon && beat.is_positive() {
        let stroke = crate::view::patterns::beacon::stroke(ui.ctx());
        crate::view::patterns::beacon::frame(ui.painter(), beat, stroke);
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("drum-beacon"), beat));
    }
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new("drum-grid-bounds"),
            (area, ui.clip_rect(), cells),
        )
    });
}
