//! The whole Song, or the open scene, above its grid; a click plays from that point.
use super::*;
use crate::view::timeline_minimap;

pub fn show(a: &mut App, ui: &mut egui::Ui, p: Pane) {
    let id = scene_id(a, p);
    if p == Pane::Scene && id.is_none() {
        return;
    }
    let Some(l) = &a.loaded else { return };
    let music = &l.music;
    let Ok((occurrences, end)) = model::occurrences(music, id) else {
        return;
    };
    let catalog = Catalog::read(music).unwrap_or_default();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width(),
            timeline_minimap::height(ui.available_height()),
        ),
        egui::Sense::click(),
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), "minimap")), rect));
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, timeline_minimap::BACKGROUND);
    let end = end.max(1) as f32;
    let x = |tick: u64| rect.left() + tick as f32 / end * rect.width();
    for &(pattern, start) in &occurrences {
        let Some(def) = catalog.patterns.get(&pattern) else {
            continue;
        };
        let Ok(length) = def.length() else { continue };
        let area = egui::Rect::from_x_y_ranges(x(start)..=x(start + length), rect.y_range());
        let color = super::super::cards::minimap_color(&l.keys);
        super::super::cards::paint_notes(&painter, area, music, def, None, &color);
    }
    let playing = a
        .playback
        .active
        .as_ref()
        .filter(|active| active.mode == crate::play::playback::Mode::Arrangement(id))
        .and_then(|active| crate::play::transport::tick_for(a, active.clock_track));
    if let Some(tick) = playing {
        painter.vline(
            x(tick).min(rect.right()),
            rect.y_range(),
            egui::Stroke::new(2.0_f32, crate::view::timeline_repeat::PLAYHEAD),
        );
    }
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let tick = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * end;
        crate::play::arrangement::start_from(a, id, tick.round() as u64);
    }
}
