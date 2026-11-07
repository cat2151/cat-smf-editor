use super::*;
use crate::view::piano_roll;
use crate::view::timeline_follow::Follow;
use crate::view::timeline_repeat as repeat;
use eframe::egui::Color32;

use super::drawing::*;

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
    if loaded.music.tracks.is_empty() {
        return;
    }
    let tick = crate::play::playback::location(app).0;
    let panes = if super::song::enabled(app) {
        match super::panes::Panes::song(app, tick) {
            Ok(panes) => panes,
            Err(error) => {
                app.load_error = Some(error);
                return;
            }
        }
    } else {
        let Some(panes) = super::panes::Panes::read(app, tick) else {
            return;
        };
        panes
    };
    let super::panes::Panes {
        mut overview,
        mut layouts,
        kinds,
        tracks,
        song,
        curves,
    } = panes;
    let drums: Vec<_> = tracks
        .iter()
        .map(|&t| loaded.patches[t].sequencer())
        .collect();
    let playing = if song.is_some() {
        super::song_playback::playing(app)
    } else {
        crate::play::transport::tick_for(app, app.track)
    };
    let mut following = Follow::new(ctx, app.playback.follow, playing);
    let minimap = piano_roll::minimap::Data::read(loaded, app.track);
    if song.is_none() {
        minimap.extend_timeline(loaded, app.track, &mut overview);
        for (layout, &track) in layouts.iter_mut().zip(&tracks) {
            minimap.extend_timeline(loaded, track, layout);
        }
    }
    let range = app.playback.repeat.filter(|_| {
        song.is_none()
            || app.playback.repeat_mode == Some(crate::play::playback::Mode::Arrangement(None))
    });
    let screen = if song.is_some() {
        crate::view::timeline_controls::Screen::TrackOverviewSong
    } else {
        crate::view::timeline_controls::Screen::TrackOverview
    };
    let start_of = |track, tick| {
        song.as_ref().map_or_else(
            || measure_start(loaded, track, tick),
            |s| s.measure_start(tick),
        )
    };
    let end_of = |track, start| {
        song.as_ref().map_or_else(
            || next_measure(loaded, track, start),
            |s| {
                let map = s.time.map(0).unwrap();
                map.tick_at(&super::measure(map.position(start).measure + 1))
                    .unwrap_or(start)
            },
        )
    };
    let state = &mut app.track_overview;
    state.viewport.begin(screen);
    state.cell.0 = state.cell.0.min(tracks.len() - 1);
    if let Some(song) = &song {
        state.cell.1 = song.measure_start(state.cell.1.min(song.end.saturating_sub(1)));
    }
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
                    let end = end_of(track, start);
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
                        draw_pane(&painter, rect, layout, kinds[slot], drums[slot], color);
                        // Tempo events and the tempo curve live on the conductor track.
                        if track == 0 {
                            crate::view::timeline_minimap::tempos::draw(
                                &painter,
                                rect,
                                rect.width() / layout.scale,
                                minimap.layers(),
                            );
                        }
                        super::curves::draw(&painter, rect, layout.scale, &curves[slot]);
                        if let Some(song) = &song {
                            super::drawing::patterns(
                                &painter,
                                rect,
                                layout.scale,
                                &song.rows[slot],
                            );
                        }
                        if state.cell.0 == slot {
                            let start = state.cell.1;
                            let end = end_of(track, start);
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
                            start_of(track, tick)
                        };
                        if response.clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            action = Some(Action::Play(slot, at(pos)));
                        }
                        if response.secondary_clicked()
                            && let Some(pos) = response.interact_pointer_pos()
                        {
                            state.menu = Some((slot, at(pos)));
                        }
                        response.context_menu(|ui| {
                            let Some((track, tick)) = state.menu else {
                                ui.close_menu();
                                return;
                            };
                            if ui
                                .button(if song.is_some() {
                                    "t — Container play"
                                } else {
                                    "t — track play"
                                })
                                .clicked()
                            {
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
                            state.cell.0 == slot,
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
        crate::view::timeline_domain::click(screen, app, app.track, tick);
    }
    if let Some(action) = repeat_action {
        if song.is_some() {
            super::song_playback::repeat(app, action);
        } else {
            repeat::apply(app, action);
        }
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
