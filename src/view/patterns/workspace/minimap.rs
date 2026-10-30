//! The whole Song, or the open scene, above its grid; a click plays from that point.
use super::*;
use crate::view::timeline_minimap;

pub fn show(a: &mut App, ui: &mut egui::Ui, p: Pane) {
    if p == Pane::Song {
        super::timeline::show(a, ui);
        return;
    }
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
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(
            ui.available_width(),
            timeline_minimap::height(ui.available_height()),
        ),
        egui::Sense::hover(),
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), "minimap")), rect));
    let playing = a
        .playback
        .active
        .as_ref()
        .filter(|active| active.mode == crate::play::playback::Mode::Arrangement(id))
        .and_then(|active| crate::play::transport::tick_for(a, active.clock_track));
    let state = &mut a.patterns.workspace.scene_timeline;
    if state.scope != id {
        *state = Default::default();
        state.scope = id;
    }
    state
        .viewport
        .begin(crate::view::timeline_controls::Screen::Scene);
    let Some((stride, label)) = state.cells else {
        return;
    };
    let collection = scene::read(music).unwrap_or_default();
    let Some(scene) = collection.scenes.iter().find(|s| Some(s.id) == id) else {
        return;
    };
    let mut boundaries = vec![0];
    for meter in &scene.meters {
        boundaries.push(boundaries.last().unwrap() + meter.ticks);
    }
    let axis = super::timeline::SceneCells {
        boundaries: &boundaries,
        stride,
        label,
    };
    let binding = state.viewport.binding(&axis, end);
    let end = end.max(1) as f32;
    let navigation =
        state
            .viewport
            .minimap_state
            .show_on(ui, rect, &binding, playing, |painter, rect| {
                let x = |tick: u64| rect.left() + tick as f32 / end * rect.width();
                for &(pattern, start) in &occurrences {
                    let Some(def) = catalog.patterns.get(&pattern) else {
                        continue;
                    };
                    let Ok(length) = def.length() else { continue };
                    let area =
                        egui::Rect::from_x_y_ranges(x(start)..=x(start + length), rect.y_range());
                    let color = super::super::cards::minimap_color(&l.keys);
                    super::super::cards::paint_notes(painter, area, music, def, None, &color);
                }
            });
    if let Some(navigation) = navigation {
        state.viewport.pending = Some(egui::vec2(navigation.scroll, 0.0));
        a.playback.follow = false;
        if let Some(tick) = navigation.seek {
            crate::view::timeline_domain::click(
                crate::view::timeline_controls::Screen::Scene,
                a,
                0,
                tick,
            );
        }
    }
}
