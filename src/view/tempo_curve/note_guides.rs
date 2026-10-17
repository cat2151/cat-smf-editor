//! Nearest note-event times bracketing the pointer in the tempo panel.
use super::{Loaded, Roll};
use cat_smf_editor::score::note_view::{note_off, note_on};
use eframe::egui;

pub(super) const OFF_COLOR: egui::Color32 = egui::Color32::from_gray(190);

pub(super) fn draw(ui: &egui::Ui, roll: &Roll, loaded: &Loaded) {
    if !ui.rect_contains_pointer(roll.rect) {
        return;
    }
    let Some(pointer) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    let mut left: Option<(f32, bool, u64)> = None;
    let mut right: Option<(f32, bool, u64)> = None;
    for event in loaded.music.tracks.iter().flat_map(|t| &t.events) {
        let on = note_on(event);
        if !on && !note_off(event) {
            continue;
        }
        let x = roll.rect.left() + event.tick as f32 * roll.scale;
        // An exact event position starts the interval to its right.
        let (candidate, nearer) = if x <= pointer.x {
            (&mut left, true)
        } else {
            (&mut right, false)
        };
        if candidate.is_none_or(|(old, was_on, _)| {
            (if nearer { x > old } else { x < old }) || x == old && on && !was_on
        }) {
            *candidate = Some((x, on, event.tick));
        }
    }
    let painter = ui
        .painter()
        .with_clip_rect(ui.clip_rect().intersect(roll.rect));
    for (x, on, tick) in left.into_iter().chain(right) {
        if on {
            let beat = loaded.time.map(0).map_or(1, |map| map.position(tick).beat);
            let base = crate::view::row::beat_color(beat);
            let dim = |value: u8| (f32::from(value) * 0.55) as u8;
            let color = egui::Color32::from_rgb(dim(base.r()), dim(base.g()), dim(base.b()));
            painter.extend(egui::Shape::dashed_line(
                &[
                    egui::pos2(x, roll.rect.top()),
                    egui::pos2(x, roll.rect.bottom()),
                ],
                egui::Stroke::new(1.5_f32, color),
                5.0,
                4.0,
            ));
        } else {
            painter.vline(
                x,
                roll.rect.y_range(),
                egui::Stroke::new(1.5_f32, OFF_COLOR),
            );
        }
    }
}
