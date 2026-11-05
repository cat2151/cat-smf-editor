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
/// The Note Ons of a pattern, for painting `range` of it.
struct Shown {
    events: Vec<Event>,
    durations: Vec<Option<u64>>,
    range: (u64, u64),
}
/// A note's part of the shown range, as fractions of its width.
struct Span<'e> {
    event: &'e Event,
    left: f32,
    width: f32,
    /// The Note On itself is in the range, not only the tail of an earlier one.
    onset: bool,
}
impl Shown {
    fn read(music: &Music, p: &Definition, measures: Option<(usize, usize)>) -> Option<Self> {
        let full = p.range().ok()?;
        let events = body::extract(&music.tracks[p.track].events, full).ok()?;
        let offset = |n: usize| p.meters.iter().take(n).map(|m| m.ticks).sum::<u64>();
        let range = measures.map_or(full, |(first, end)| {
            (full.0 + offset(first), full.0 + offset(end))
        });
        let durations = cat_smf_editor::score::note_view::NoteView::new(&events).durations;
        (range.1 > range.0).then_some(Self {
            events,
            durations,
            range,
        })
    }
    fn spans(&self) -> impl Iterator<Item = Span<'_>> {
        let (start, end) = self.range;
        let length = (end - start) as f32;
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| note_on(e))
            .filter_map(move |(index, e)| {
                let stop = e.tick + self.durations[index].unwrap_or(0);
                if e.tick >= end || stop <= start && e.tick < start {
                    return None;
                }
                Some(Span {
                    event: e,
                    left: (e.tick.max(start) - start) as f32 / length,
                    width: (stop.min(end) - e.tick.max(start)) as f32 / length,
                    onset: e.tick >= start,
                })
            })
    }
    /// Lowest and highest pitch of the whole pattern, shown or not.
    fn pitches(&self) -> Option<(u8, u8)> {
        let pitches = || self.events.iter().filter(|e| note_on(e)).map(|e| e.data[0]);
        Some((pitches().min()?, pitches().max()?))
    }
}
const NOTE_HEIGHT: f32 = 3.0;
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
    let Some(shown) = Shown::read(music, p, measures) else {
        return;
    };
    for n in shown.spans() {
        let x = area.left() + n.left * area.width();
        let y = area.bottom() - f32::from(n.event.data[0]) / 127.0 * area.height();
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2((n.width * area.width()).max(2.0), NOTE_HEIGHT),
            ),
            0.0,
            color(p.track, n.event),
        );
    }
}
/// Width of the dense mark at a Note On.
const ONSET_WIDTH: f32 = 2.0;
/// Opacity of a note past its Note On, so repeated notes read apart from a held one.
const HELD_OPACITY: f32 = 0.45;
/// `measures` of a pattern in its container's `color`: the pattern's lowest and highest
/// notes touch the bottom and top of `area` whichever measures are shown.
pub(super) fn paint_container_notes(
    painter: &egui::Painter,
    area: egui::Rect,
    music: &Music,
    p: &Definition,
    measures: Option<(usize, usize)>,
    color: egui::Color32,
) {
    let Some(shown) = Shown::read(music, p, measures) else {
        return;
    };
    let Some((low, high)) = shown.pitches() else {
        return;
    };
    let travel = area.height() - NOTE_HEIGHT;
    for n in shown.spans() {
        let height = if high == low {
            0.5
        } else {
            f32::from(n.event.data[0] - low) / f32::from(high - low)
        };
        let min = egui::pos2(
            area.left() + n.left * area.width(),
            area.bottom() - NOTE_HEIGHT - height * travel,
        );
        let size = |width: f32| egui::vec2(width.max(ONSET_WIDTH), NOTE_HEIGHT);
        let held = egui::Rect::from_min_size(min, size(n.width * area.width()));
        painter.rect_filled(held, 0.0, color.gamma_multiply(HELD_OPACITY));
        if n.onset {
            let onset = egui::Rect::from_min_size(min, size(ONSET_WIDTH));
            painter.rect_filled(onset, 0.0, color);
        }
    }
}
