//! Fixed whole-Song overview, with zoom and repeat gestures for the timeline.
use super::*;
use crate::view::{timeline_minimap, timeline_repeat as repeat, timeline_zoom};
mod axis;
pub(super) use axis::{scene_boundaries, Cells, SceneCells};

/// Width of a Song measure at zoom 1, and the narrowest it is drawn.
pub(super) const CELL: f32 = 90.0;
const MIN_CELL: f32 = 2.0;

#[derive(Default)]
pub(super) struct State {
    pub(super) viewport: crate::view::timeline_controls::Viewport,
    pub(super) repeat: repeat::State,
    pub(super) cells: Option<(f32, f32)>,
    pub(super) scope: Option<u64>,
    pub(super) anchor: Option<u64>,
}

impl State {
    pub(super) fn suspend(&mut self) {
        self.viewport.suspend();
        self.repeat = Default::default();
    }

    pub(super) fn zoom(&self) -> f32 {
        self.viewport.zoom().x
    }
}

pub(super) fn show(a: &mut App, ui: &mut egui::Ui) {
    let Some(l) = &a.loaded else { return };
    let Some((catalog, collection)) = &l.arrangement else {
        return;
    };
    let Ok((occurrences, end)) = model::occurrences_in(&l.music, catalog, collection, None) else {
        return;
    };
    let time = cat_smf_editor::score::musical_time::MusicalTime::new(&l.music);
    let Some(map) = time.map(0) else { return };
    let playing = a
        .playback
        .active
        .as_ref()
        .filter(|active| active.mode == crate::play::playback::Mode::Arrangement(None))
        .and_then(|active| crate::play::transport::tick_for(a, active.clock_track));
    let state = &mut a.patterns.workspace.timeline;
    state
        .viewport
        .begin(crate::view::timeline_controls::Screen::Song);
    state.viewport.min_time_zoom = MIN_CELL / CELL;
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
        if let Some((stride, label)) = state.cells {
            let axis = Cells { map, stride, label };
            let binding = state.viewport.binding(&axis, end);
            let action = state.viewport.minimap_state.show_on(
                ui,
                rect,
                &binding,
                playing,
                |painter, rect| {
                    let x = |tick: u64| rect.left() + tick as f32 * scale;
                    for &(pattern, start) in &occurrences {
                        let Some(def) = catalog.patterns.get(&pattern) else {
                            continue;
                        };
                        let Ok(length) = def.length() else { continue };
                        let part = egui::Rect::from_x_y_ranges(
                            x(start)..=x(start + length),
                            rect.y_range(),
                        );
                        let color = super::super::cards::minimap_color(&l.keys);
                        super::super::cards::paint_notes(
                            painter, part, &l.music, def, None, &color,
                        );
                    }
                },
            );
            #[cfg(test)]
            if let Some(visible) = ui.ctx().data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new((
                    crate::view::timeline_controls::Screen::Song,
                    "visible",
                )))
            }) {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new((Pane::Song.title(), "minimap-visible")),
                        visible,
                    )
                });
            }
            if let Some(action) = action {
                state.viewport.pending = Some(egui::vec2(action.scroll, 0.0));
                seek = action.seek;
                navigated = true;
            }
        }
    });
    if navigated {
        a.playback.follow = false;
    }
    if let Some(tick) = seek {
        crate::view::timeline_domain::click(
            crate::view::timeline_controls::Screen::Song,
            a,
            0,
            tick,
        );
    }
}

pub(super) fn apply(a: &mut App, action: repeat::Action) {
    a.patterns.workspace.timeline.anchor = None;
    match action {
        repeat::Action::Start(range) => crate::play::transport::repeat::start_song(a, range),
        other => repeat::apply(a, other),
    }
}

pub(super) fn collect(app: &App, out: &mut Vec<Command>) {
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
        (
            "song-repeat-a",
            "Song A-B Repeat A：選択小節の先頭を開始点に設定",
            false,
        ),
        (
            "song-repeat-b",
            "Song A-B Repeat B：選択小節の末尾を終了点に設定して再生",
            true,
        ),
    ] {
        if end && end_range(app).is_none() {
            continue;
        }
        out.push(Command::plain(Screen::Song, id, name, &[], move |a, _| {
            let Some(l) = &a.loaded else { return };
            let Ok(tick) =
                model::measure_tick(&l.music, None, a.patterns.workspace.song_cell.measure)
            else {
                return;
            };
            if end {
                if let Some(range) = end_range(a) {
                    crate::play::transport::repeat::start_song(a, range);
                }
            } else {
                a.patterns.workspace.timeline.anchor = Some(tick);
            }
        }));
    }
    out.push(Command::plain(
        Screen::Song,
        "song-repeat-clear",
        "Song A-B Repeat 解除",
        &[],
        |a, _| {
            a.patterns.workspace.timeline.anchor = None;
            crate::play::transport::repeat::clear(a);
        },
    ));
}

fn end_range(a: &App) -> Option<(u64, u64)> {
    let music = &a.loaded.as_ref()?.music;
    let next = a.patterns.workspace.song_cell.measure.checked_add(1)?;
    let tick = model::measure_tick(music, None, next).ok()?;
    let start = a
        .patterns
        .workspace
        .timeline
        .anchor
        .or(a.playback.repeat.map(|r| r.0))?;
    (start < tick).then_some((start, tick))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_end_includes_the_selected_measure_with_changing_meters() {
        let (mut a, _, _, _, _) = crate::view::patterns::workspace_tests::verification_song();
        let music = &mut a.loaded.as_mut().unwrap().music;
        let events = &mut music.tracks[0].events;
        events.retain(|event| !(event.status == 0xff && event.meta_type == 0x58));
        events.extend([(0, 4, 2), (1920, 3, 2), (3360, 5, 3)].map(
            |(tick, numerator, exponent)| cat_smf_editor::model::Event {
                tick,
                delta: 0,
                status: 0xff,
                meta_type: 0x58,
                data: vec![numerator, exponent, 24, 8],
            },
        ));
        events.sort_by_key(|event| event.tick);
        a.patterns.workspace.timeline.anchor = Some(0);
        for (measure, end) in [(1, 1920), (2, 3360), (3, 4560)] {
            a.patterns.workspace.song_cell.measure = measure;
            assert_eq!(end_range(&a), Some((0, end)));
        }
        a.patterns.workspace.timeline.anchor = Some(3360);
        for measure in [1, 2] {
            a.patterns.workspace.song_cell.measure = measure;
            assert_eq!(end_range(&a), None, "B must follow A");
        }
        a.patterns.workspace.song_cell.measure = 3;
        assert_eq!(end_range(&a), Some((3360, 4560)));
        a.patterns.workspace.song_cell.measure = u64::MAX;
        assert_eq!(end_range(&a), None);
    }
}
