use cat_smf_editor::{
    model::{Event, Music},
    score::{
        key_signature::KeySignatures,
        note_view::note_on,
        pattern::{body, Catalog, Definition},
        scene::Scene,
    },
};
/// Note colors of the timeline minimaps, from each container track's key.
pub(super) fn minimap_color(keys: &KeySignatures) -> impl Fn(usize, &Event) -> egui::Color32 + '_ {
    move |track, e| {
        crate::view::timeline_minimap::note_color(
            e.data[0],
            keys.map(track).and_then(|k| k.at(e.tick)),
        )
    }
}
/// Paint every pattern of `scene` over its `measures` (scene-relative, half-open), each measure as wide as its ticks.
pub(super) fn paint_scene(
    painter: &egui::Painter,
    area: egui::Rect,
    music: &Music,
    catalog: &Catalog,
    scene: &Scene,
    measures: Option<(usize, usize)>,
    color: &dyn Fn(usize, &Event) -> egui::Color32,
) {
    let count = scene.meters.len();
    let (first, end) = measures.map_or((0, count), |(first, end)| (first, end.min(count)));
    let shown = scene.meters.get(first..end).unwrap_or_default();
    let total = shown.iter().map(|m| m.ticks).sum::<u64>() as f32;
    if total <= 0.0 {
        return;
    }
    let mut left = area.left();
    for (measure, meter) in (first..).zip(shown) {
        let right = left + meter.ticks as f32 / total * area.width();
        let slot = egui::Rect::from_x_y_ranges(left..=right, area.y_range());
        for cell in &scene.cells {
            let Some(p) = catalog.patterns.get(&cell.pattern) else {
                continue;
            };
            let start = cell.measure as usize;
            if (start..start + p.meters.len()).contains(&measure) {
                let k = measure - start;
                paint_notes(painter, slot, music, p, Some((k, k + 1)), color);
            }
        }
        left = right;
    }
}
use eframe::egui;
/// Paint the notes of `measures` (pattern-relative, half-open) of a pattern into `area`.
/// `color` sees the container track and each Note On at its tick there.
pub(super) fn paint_notes(
    painter: &egui::Painter,
    area: egui::Rect,
    music: &Music,
    p: &Definition,
    measures: Option<(usize, usize)>,
    color: &dyn Fn(usize, &Event) -> egui::Color32,
) {
    let Ok(full) = p.range() else { return };
    let Ok(events) = body::extract(&music.tracks[p.track].events, full) else {
        return;
    };
    let offset = |n: usize| p.meters.iter().take(n).map(|m| m.ticks).sum::<u64>();
    let range = measures.map_or(full, |(first, end)| {
        (full.0 + offset(first), full.0 + offset(end))
    });
    if range.1 <= range.0 {
        return;
    }
    let notes = cat_smf_editor::score::note_view::NoteView::new(&events);
    let span = (range.1 - range.0) as f32;
    for (index, e) in events.iter().enumerate().filter(|(_, e)| note_on(e)) {
        let end = e.tick + notes.durations[index].unwrap_or(0);
        if e.tick >= range.1 || end <= range.0 && e.tick < range.0 {
            continue;
        }
        let x = area.left() + (e.tick.max(range.0) - range.0) as f32 / span * area.width();
        let w = (end.min(range.1) - e.tick.max(range.0)) as f32 / span * area.width();
        let y = area.bottom() - f32::from(e.data[0]) / 127.0 * area.height();
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w.max(2.0), 3.0)),
            0.0,
            color(p.track, e),
        );
    }
}
