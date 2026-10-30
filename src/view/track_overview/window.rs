use super::*;
use crate::view::piano_roll::{self, Layout};
use crate::view::timeline_follow::Follow;
use crate::view::timeline_repeat as repeat;
use eframe::egui::Color32;
use eframe::emath::GuiRounding;

const NAMES_WIDTH: f32 = 120.0;
const EMPTY_FILL: Color32 = Color32::from_gray(40);
/// Space between two tracks' panes.
const GAP: f32 = 4.0;
/// Width of the track color strip at the header's left.
const STRIP: f32 = 4.0;
const LANE_TINT: f32 = 0.12;
const NAME_ACTIVE_TINT: f32 = 0.35;
const MEASURE_TINT: f32 = 0.3;
const FRAME_TINT: f32 = 0.7;
/// A drum note past its first 32nd, so separate hits stay apart.
const TAIL_TINT: f32 = 0.35;

enum Action {
    Play(usize, u64),
    TrackPlay(usize, u64),
    Jump(usize, u64),
}

pub(crate) fn show(app: &mut App, ctx: &egui::Context) {
    if !app.track_overview.open {
        app.track_overview.viewport.suspend();
        app.track_overview.repeat_drag = Default::default();
        return;
    }
    let Some(loaded) = &app.loaded else {
        return;
    };
    let tracks: Vec<usize> = (0..loaded.music.tracks.len()).collect();
    if tracks.is_empty() {
        return;
    }
    let tick = crate::play::playback::location(app).0;
    let Some(super::panes::Panes {
        mut overview,
        mut layouts,
        kinds,
    }) = super::panes::Panes::read(app, tick)
    else {
        return;
    };
    let heads: Vec<_> = tracks.iter().map(|&t| drum_head(loaded, t)).collect();
    let curves = super::curves::read(loaded);
    let playing = crate::play::transport::tick_for(app, app.track);
    let mut following = Follow::new(ctx, app.playback.follow, playing);
    let minimap = piano_roll::minimap::Data::read(loaded, app.track);
    minimap.extend_timeline(loaded, app.track, &mut overview);
    for (layout, &track) in layouts.iter_mut().zip(&tracks) {
        minimap.extend_timeline(loaded, track, layout);
    }
    let range = app.playback.repeat;
    let state = &mut app.track_overview;
    state
        .viewport
        .begin(crate::view::timeline_controls::Screen::TrackOverview);
    state.viewport.panes = 1;
    let reveal = std::mem::take(&mut state.reveal);
    let mut action = None;
    let mut repeat_action = None;
    let mut open = true;
    placement(ctx, app.config.piano_roll_maximized)
        .open(&mut open)
        .show(ctx, |ui| {
            if let Some(error) = &minimap.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            let area = state.viewport.bars(ui, state.repeat_drag.is_none());
            let time_scale = overview.scale * state.viewport.zoom().x;
            let timeline = state.viewport.binding(
                &time_scale,
                (overview.size.x / overview.scale).round() as u64,
            );
            let (area, navigated) = state.viewport.minimap_on(
                ui,
                area,
                &overview,
                minimap.layers(),
                playing,
                &timeline,
            );
            following.minimap(&mut app.playback.follow, navigated);
            let names = area.with_max_x(area.left() + NAMES_WIDTH);
            let area = area.with_min_x(names.right());
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                ui.spacing_mut().scroll.floating = false;
                let bar = ui.spacing().scroll.allocated_width();
                for layout in &mut layouts {
                    state.viewport.scale_layout(layout);
                }
                let pane_height = layouts[0].size.y;
                let shown = egui::vec2(area.width() - bar, area.height() - bar - repeat::HEIGHT);
                let mut scrollbar =
                    egui::Rect::from_min_max(area.min, area.max - egui::Vec2::splat(bar))
                        .shrink2(egui::Vec2::splat(crate::view::timeline_zoom::BUTTON));
                scrollbar.min.y += repeat::HEIGHT;
                let mut scroll = egui::ScrollArea::both()
                    .id_salt("track-overview")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .scroll_bar_rect(scrollbar)
                    .drag_to_scroll(false);
                let pending = state.viewport.pending.take();
                let current = pending.unwrap_or(state.viewport.scroll);
                let scale = layouts[0].scale;
                if let Some(tick) = following.tick() {
                    scroll = scroll.scroll_offset(egui::vec2(
                        (tick as f32 * scale - shown.x / 2.0).max(0.0),
                        current.y,
                    ));
                } else if reveal && !navigated {
                    let (track, start) = state.cell;
                    let end = next_measure(loaded, track, start);
                    let cell = egui::Rect::from_x_y_ranges(
                        start as f32 * scale..=end as f32 * scale,
                        track as f32 * pane_height..=(track + 1) as f32 * pane_height,
                    );
                    scroll = scroll.scroll_offset(reveal_offset(current, shown, cell));
                } else if let Some(offset) = pending {
                    scroll = scroll.scroll_offset(offset);
                }
                let output = scroll.show_viewport(ui, |ui, visible| {
                    let width = layouts
                        .iter()
                        .map(|l| l.size.x)
                        .fold(visible.width(), f32::max);
                    let (whole, _) = ui.allocate_exact_size(
                        egui::vec2(width, pane_height * tracks.len() as f32 + repeat::HEIGHT),
                        egui::Sense::hover(),
                    );
                    let screen = egui::Rect::from_min_max(
                        whole.min + visible.min.to_vec2(),
                        whole.min + visible.max.to_vec2(),
                    );
                    let row = egui::Rect::from_min_size(
                        egui::pos2(whole.left(), screen.top()),
                        egui::vec2(width, repeat::HEIGHT),
                    );
                    repeat_action = timeline.repeat(ui, row, range, &mut state.repeat_drag);
                    let panes = screen.with_min_y(screen.top() + repeat::HEIGHT);
                    for (slot, layout) in layouts.iter().enumerate() {
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(
                                whole.left(),
                                whole.top() + repeat::HEIGHT + slot as f32 * pane_height,
                            ),
                            egui::vec2(width, pane_height),
                        );
                        let clip = rect.intersect(panes);
                        if clip.height() <= 0.0 {
                            continue;
                        }
                        let painter = ui.painter_at(clip);
                        let track = tracks[slot];
                        let color = crate::view::track_color(track);
                        draw_pane(&painter, rect, layout, kinds[slot], heads[slot], color);
                        // Tempo events and the tempo curve live on the conductor track.
                        if track == 0 {
                            crate::view::timeline_minimap::tempos::draw(
                                &painter,
                                rect,
                                rect.width() / layout.scale,
                                minimap.layers(),
                            );
                        }
                        super::curves::draw(&painter, rect, layout.scale, &curves[track]);
                        if state.cell.0 == track {
                            let start = state.cell.1;
                            let end = next_measure(loaded, track, start);
                            painter.rect_stroke(
                                egui::Rect::from_x_y_ranges(
                                    rect.left() + start as f32 * layout.scale
                                        ..=rect.left() + end as f32 * layout.scale,
                                    lane(rect).y_range(),
                                ),
                                0.0,
                                // White, since orange is one of the track colors.
                                egui::Stroke::new(2.0_f32, Color32::WHITE),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let response = ui.interact(
                            clip,
                            crate::view::command_input::preview_id(
                                ui.ctx(),
                                egui::Id::new(("track-overview-pane", slot)),
                            ),
                            egui::Sense::click(),
                        );
                        let at = |pos: egui::Pos2| {
                            let tick = ((pos.x - rect.left()) / layout.scale).max(0.0) as u64;
                            measure_start(loaded, track, tick)
                        };
                        if response.clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            action = Some(Action::Play(track, at(pos)));
                        }
                        if response.secondary_clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            state.menu = Some((track, at(pos)));
                        }
                        response.context_menu(|ui| {
                            let Some((track, tick)) = state.menu else {
                                ui.close_menu();
                                return;
                            };
                            if ui.button("t — track play").clicked() {
                                action = Some(Action::TrackPlay(track, tick));
                                ui.close_menu();
                            }
                            if ui.button("g — jump").clicked() {
                                action = Some(Action::Jump(track, tick));
                                ui.close_menu();
                            }
                        });
                        let header = egui::Rect::from_x_y_ranges(names.x_range(), rect.y_range());
                        // Outside the scroll area, whose clip would hide the names.
                        draw_name(
                            &egui::Painter::new(
                                ui.ctx().clone(),
                                ui.layer_id(),
                                egui::Rect::from_x_y_ranges(names.x_range(), clip.y_range()),
                            ),
                            header,
                            &loaded.track_titles[track],
                            color,
                            state.cell.0 == track,
                        );
                    }
                    if let Some(tick) = playing {
                        ui.painter().vline(
                            whole.left() + tick as f32 * scale,
                            panes.y_range(),
                            egui::Stroke::new(2.0_f32, repeat::PLAYHEAD),
                        );
                    }
                });
                let panel = output
                    .inner_rect
                    .with_min_y(output.inner_rect.top() + repeat::HEIGHT);
                state.viewport.finish(ui, area, panel, output.state.offset);
            });
        });
    if !open {
        app.track_overview.open = false;
    }
    if let Some(tick) = app.track_overview.viewport.seek.take() {
        crate::view::timeline_domain::click(
            crate::view::timeline_controls::Screen::TrackOverview,
            app,
            app.track,
            tick,
        );
    }
    if let Some(action) = repeat_action {
        repeat::apply(app, action);
    }
    match action {
        Some(Action::Play(track, tick)) => play(app, track, tick),
        Some(Action::TrackPlay(track, tick)) => play_track(app, track, tick),
        Some(Action::Jump(track, tick)) => jump(app, track, tick),
        None => {}
    }
}

fn next_measure(loaded: &Loaded, track: usize, start: u64) -> u64 {
    loaded
        .time
        .map(track)
        .and_then(|map| map.tick_at(&super::measure(map.position(start).measure + 1)))
        .unwrap_or(start)
}

/// The smallest scroll change that brings `cell` inside the `shown` size.
pub(super) fn reveal_offset(
    mut offset: egui::Vec2,
    shown: egui::Vec2,
    cell: egui::Rect,
) -> egui::Vec2 {
    for axis in 0..2 {
        if cell.max[axis] > offset[axis] + shown[axis] {
            offset[axis] = cell.max[axis] - shown[axis];
        }
        if cell.min[axis] < offset[axis] {
            offset[axis] = cell.min[axis];
        }
        offset[axis] = offset[axis].max(0.0);
    }
    offset
}

fn drum_head(loaded: &Loaded, track: usize) -> Option<u64> {
    loaded.patches[track].drums.as_ref()?;
    let sixteenth = cat_smf_editor::score::drums::tick(loaded.music.division, 1).ok()?;
    Some(sixteenth / 2)
}

/// The track's own color on fill, frame, measure lines and notes; an empty track stays gray.
fn draw_pane(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &Layout,
    kind: Kind,
    head: Option<u64>,
    color: Color32,
) {
    let lane = lane(rect);
    // The lane spans the whole song; shapes that long lose precision and flicker while scrolling.
    let shown = lane
        .intersect(painter.clip_rect())
        .round_to_pixels(painter.pixels_per_point());
    if !shown.is_positive() {
        return;
    }
    if kind == Kind::Empty {
        painter.rect_filled(shown, 0.0, EMPTY_FILL);
        return;
    }
    painter.rect_filled(shown, 0.0, color.gamma_multiply(LANE_TINT));
    for &(x, beat) in &layout.beats {
        let x = rect.left() + x;
        if beat == 1 && shown.x_range().contains(x) {
            painter.vline(
                x,
                shown.y_range(),
                egui::Stroke::new(1.0_f32, color.gamma_multiply(MEASURE_TINT)),
            );
        }
    }
    let origin = rect.min.to_vec2();
    for &(_, note) in &layout.notes {
        let note = note.translate(origin);
        let split = head.map_or(note.right(), |ticks| {
            note.right().min(note.left() + ticks as f32 * layout.scale)
        });
        for (part, fill) in [
            (note.with_max_x(split), color),
            (note.with_min_x(split), color.gamma_multiply(TAIL_TINT)),
        ] {
            let part = part.intersect(shown);
            if part.is_positive() {
                painter.rect_filled(part, 0.0, fill);
            }
        }
    }
    let frame = egui::Stroke::new(1.0_f32, color.gamma_multiply(FRAME_TINT));
    painter.hline(shown.x_range(), shown.top() + 0.5, frame);
    painter.hline(shown.x_range(), shown.bottom() - 0.5, frame);
    for x in [lane.left() + 0.5, lane.right() - 0.5] {
        if shown.x_range().contains(x) {
            painter.vline(x, shown.y_range(), frame);
        }
    }
}

/// The pane less the gap that separates it from its neighbors.
fn lane(rect: egui::Rect) -> egui::Rect {
    rect.shrink2(egui::vec2(0.0, GAP / 2.0))
}

/// DAW-like track header: a color strip and the name; the cell's track is filled brighter.
fn draw_name(painter: &egui::Painter, rect: egui::Rect, name: &str, color: Color32, active: bool) {
    let lane = lane(rect);
    let fill = if active { NAME_ACTIVE_TINT } else { LANE_TINT };
    painter.rect_filled(lane, 0.0, color.gamma_multiply(fill));
    painter.rect_filled(lane.with_max_x(lane.left() + STRIP), 0.0, color);
    painter.text(
        egui::pos2(lane.left() + STRIP + 4.0, lane.top() + 2.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(12.0),
        if active { Color32::WHITE } else { color },
    );
}

fn placement(ctx: &egui::Context, maximized: bool) -> egui::Window<'static> {
    if crate::view::command_input::preview_active(ctx) {
        return crate::view::command_input::preview_window(ctx, "Track Overview");
    }
    let window = egui::Window::new("Track Overview").collapsible(false);
    if maximized {
        window
            .id(egui::Id::new("track-overview-maximized"))
            .title_bar(false)
            .frame(egui::Frame::NONE.fill(ctx.style().visuals.window_fill()))
            .fixed_rect(ctx.screen_rect())
    } else {
        window.default_size([900.0, 480.0]).min_height(180.0)
    }
}
