//! Fixed whole-Song overview, with zoom and repeat gestures for the timeline.
use super::*;
use crate::view::{timeline_minimap, timeline_repeat as repeat, timeline_zoom};
mod axis;
pub(super) use axis::Cells;

#[derive(Default)]
pub(super) struct State {
    pub(super) viewport: crate::view::piano_roll::viewport::Viewport,
    pub(super) repeat: repeat::State,
    pub(super) cells: Option<(f32, f32)>,
    drag: Option<f32>,
    anchor: Option<u64>,
}

impl State {
    pub(super) fn zoom(&self) -> f32 {
        self.viewport.zoom().x
    }
}

pub(super) fn show(a: &mut App, ui: &mut egui::Ui) {
    use crate::view::timeline_repeat::TickAxis;
    let Some(l) = &a.loaded else { return };
    let Ok((occurrences, end)) = model::occurrences(&l.music, None) else {
        return;
    };
    let catalog = Catalog::read(&l.music).unwrap_or_default();
    let time = cat_smf_editor::score::musical_time::MusicalTime::new(&l.music);
    let Some(map) = time.map(0) else { return };
    let playing = a
        .playback
        .active
        .as_ref()
        .filter(|active| active.mode == crate::play::playback::Mode::Arrangement(None))
        .and_then(|active| crate::play::transport::tick_for(a, active.clock_track));
    let state = &mut a.patterns.workspace.timeline;
    let height = timeline_minimap::height(ui.available_height());
    let (outer, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), timeline_zoom::BAR + height),
        egui::Sense::hover(),
    );
    let mut seek = None;
    let mut navigated = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(outer), |ui| {
        let rect = state.viewport.horizontal_bar(ui, state.repeat.is_none());
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new((Pane::Song.title(), "zoom")),
                outer.with_max_y(rect.top()),
            );
            d.insert_temp(egui::Id::new((Pane::Song.title(), "minimap")), rect);
        });
        let scale = rect.width().max(1.0) / end.max(1) as f32;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, timeline_minimap::BACKGROUND);
        let x = |tick: u64| rect.left() + tick as f32 * scale;
        for &(pattern, start) in &occurrences {
            let Some(def) = catalog.patterns.get(&pattern) else {
                continue;
            };
            let Ok(length) = def.length() else { continue };
            let part = egui::Rect::from_x_y_ranges(x(start)..=x(start + length), rect.y_range());
            let color = super::super::cards::minimap_color(&l.keys);
            super::super::cards::paint_notes(&painter, part, &l.music, def, None, &color);
        }
        if let Some((stride, label)) = state.cells {
            let axis = Cells { map, stride, label };
            let left = axis.tick(state.viewport.scroll.x);
            let width = state.viewport.panel.map_or(rect.width(), |p| p.width());
            let right = axis.tick(state.viewport.scroll.x + width);
            let visible = egui::Rect::from_x_y_ranges(
                x(left).clamp(rect.left(), rect.right())
                    ..=x(right).clamp(rect.left(), rect.right()),
                rect.y_range(),
            );
            painter.rect_filled(visible, 0.0, timeline_minimap::COLOR.gamma_multiply(0.12));
            painter.rect_stroke(
                visible,
                0.0,
                egui::Stroke::new(1.0_f32, timeline_minimap::COLOR),
                egui::StrokeKind::Inside,
            );
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    egui::Id::new((Pane::Song.title(), "minimap-visible")),
                    visible,
                )
            });
            let response = ui.interact(
                rect,
                ui.id().with("song-notes"),
                egui::Sense::click_and_drag(),
            );
            let local = |pos: egui::Pos2| ((pos.x - rect.left()) / scale).clamp(0.0, end as f32);
            if response.drag_started_by(egui::PointerButton::Primary) {
                state.drag = ui.input(|i| i.pointer.press_origin()).map(|pos| {
                    let start = local(pos);
                    if (left as f32..=right as f32).contains(&start) {
                        axis.x(start.round() as u64) - state.viewport.scroll.x
                    } else {
                        width / 2.0
                    }
                });
            }
            if let Some((grab, pos)) = state.drag.zip(response.interact_pointer_pos()) {
                let scroll = (axis.x(local(pos).round() as u64) - grab)
                    .clamp(0.0, (axis.x(end) - width).max(0.0));
                state.viewport.pending = Some(egui::vec2(scroll, 0.0));
                navigated = true;
            }
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let tick = local(pos).round() as u64;
                seek = Some(tick);
                navigated = true;
                state.viewport.pending =
                    Some(egui::vec2((axis.x(tick) - width / 2.0).max(0.0), 0.0));
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                state.drag = None;
            }
        }
        if let Some(tick) = playing {
            painter.vline(
                x(tick),
                rect.y_range(),
                egui::Stroke::new(2.0_f32, repeat::PLAYHEAD),
            );
        }
    });
    if navigated {
        a.playback.follow = false;
    }
    if let Some(tick) = seek {
        crate::play::arrangement::start_from(a, None, tick);
    }
}

pub(super) fn apply(a: &mut App, action: repeat::Action) {
    match action {
        repeat::Action::Start(range) => crate::play::transport::repeat::start_song(a, range),
        other => repeat::apply(a, other),
    }
}

pub(super) fn collect(out: &mut Vec<Command>) {
    for (id, name, key, enlarge) in [
        ("song-zoom-in", "時間拡大", Key::PageUp, true),
        ("song-zoom-out", "時間縮小", Key::PageDown, false),
    ] {
        out.push(
            Command::plain(Screen::Song, id, name, &[key], move |a, _| {
                a.patterns
                    .workspace
                    .timeline
                    .viewport
                    .zoom_horizontal(enlarge);
            })
            .repeating(),
        );
    }
    for (id, name, end) in [
        ("song-repeat-a", "Song repeat A", false),
        ("song-repeat-b", "Song repeat B", true),
    ] {
        out.push(Command::plain(Screen::Song, id, name, &[], move |a, _| {
            let Some(l) = &a.loaded else { return };
            let Ok(tick) =
                model::measure_tick(&l.music, None, a.patterns.workspace.song_cell.measure)
            else {
                return;
            };
            if end {
                let start = a
                    .patterns
                    .workspace
                    .timeline
                    .anchor
                    .or(a.playback.repeat.map(|r| r.0));
                if let Some(start) = start.filter(|&start| start != tick) {
                    crate::play::transport::repeat::start_song(
                        a,
                        (start.min(tick), start.max(tick)),
                    );
                }
            } else {
                a.patterns.workspace.timeline.anchor = Some(tick);
            }
        }));
    }
    out.push(Command::plain(
        Screen::Song,
        "song-repeat-clear",
        "Song repeat解除",
        &[],
        |a, _| {
            a.patterns.workspace.timeline.anchor = None;
            crate::play::transport::repeat::clear(a);
        },
    ));
}
