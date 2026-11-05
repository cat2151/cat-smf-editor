use super::{commands::Op, *};
use egui::{Color32, Rect, Stroke};

pub(super) fn cursor_stroke(ctx: &egui::Context, focused: bool) -> Stroke {
    let orange = super::super::ACTIVE_PANE;
    let color = if focused {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
        let phase = (ctx.input(|i| i.time) / 0.7).fract();
        let blend = (0.5 - 0.5 * (phase * std::f64::consts::TAU).cos()) as f32;
        orange.lerp_to_gamma(Color32::WHITE, blend)
    } else {
        orange
    };
    Stroke::new(if focused { 3.0_f32 } else { 2.0_f32 }, color)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut egui::Ui,
    music: &cat_smf_editor::model::Music,
    track: usize,
    patch: &Patch,
    state: &mut State,
    measure: &drums::Measure,
    sounding: Option<usize>,
    beacon: bool,
    focused: bool,
    action: &mut Option<Op>,
) {
    let cursor = cursor_stroke(ui.ctx(), focused);
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
    let mut playing = Rect::NOTHING;
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
        let text = egui::RichText::new(drums::NAMES[row]);
        let text = if muted {
            text.color(Color32::from_gray(100))
        } else {
            text
        };
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
            let fill = if active && muted {
                Color32::from_rgb(101, 105, 94)
            } else if active {
                Color32::from_rgb(166, 226, 46)
            } else if muted && field >= 2 {
                ui.visuals()
                    .window_fill()
                    .lerp_to_gamma(Color32::WHITE, 0.04)
            } else {
                ui.visuals().widgets.inactive.bg_fill
            };
            let mut button = egui::Button::new(label).truncate().fill(fill);
            if state.row == row && state.field == field {
                button = button.stroke(cursor);
            }
            let response = ui.put(bounds, button);
            if focused && state.row == row && state.field == field {
                // Separate the bright cursor from the green note fill.
                ui.painter().rect_stroke(
                    response.rect.shrink(cursor.width),
                    2.0,
                    Stroke::new(1.0_f32, Color32::from_gray(24)),
                    egui::StrokeKind::Inside,
                );
            }
            if (2..6).contains(&field) {
                beat = beat.union(response.rect);
            }
            if sounding.is_some_and(|step| field == step + 2) {
                playing = playing.union(response.rect);
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
    if playing.is_positive() {
        ui.painter().rect_stroke(
            playing.expand(2.0),
            2.0,
            Stroke::new(2.0_f32, crate::view::timeline_repeat::PLAYHEAD),
            egui::StrokeKind::Outside,
        );
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("drum-playing"), playing));
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
