//! The piano roll inside whatever holds it: its own window or a pane.
use super::*;
use crate::view::timeline_follow::Follow;
use crate::view::timeline_repeat as repeat;

/// What one frame of the roll asks the app to do once drawing is over.
#[derive(Default)]
pub(super) struct Outcome {
    pub released: Option<(usize, usize, u64, hover::Part, i64)>,
    pub repeat_action: Option<repeat::Action>,
    pub audition: Option<(usize, usize, u64)>,
    pub trill_click: Option<(usize, usize)>,
    pub key: Option<(usize, u8)>,
}

/// The on-screen keys sound only while no menu or input sits over the roll.
pub(super) fn keyboard_enabled(app: &App) -> bool {
    !(app.comma_menu.open
        || app.edit_menu.open
        || app.playback.open
        || app.playback.mixer_focused()
        || app.player_selection.open
        || app.track_view.go_open
        || app.trill_edit.is_some()
        || app.track_view.split_open)
}

/// Draws the roll into `ui`. `None` when no file is loaded.
pub(super) fn show(app: &mut App, ui: &mut egui::Ui) -> Option<Outcome> {
    let ctx = ui.ctx().clone();
    let loaded = app.loaded.as_ref()?;
    let keyboard_enabled = keyboard_enabled(app);
    let sounding = crate::play::playback::performance(app);
    let tracks = panes::tracks(app, loaded, &sounding);
    let active_slot = if tracks.first() == tracks.last() {
        app.track_view.active
    } else {
        tracks
            .iter()
            .position(|&track| track == app.track)
            .unwrap_or(0)
    };
    let tick = crate::play::playback::location(app).0;
    let mut overview = roll(loaded, &sounding, &tracks, app.track, tick);
    let mut layouts: Vec<_> = tracks
        .iter()
        .map(|&track| roll(loaded, &sounding, &[track], track, tick))
        .collect();
    drop(sounding);
    let playing = crate::play::transport::display_tick_for(app, app.track);
    let mut following = Follow::new(&ctx, app.playback.follow, playing);
    let minimap = minimap::Data::read(loaded, app.track);
    minimap.extend_timeline(loaded, app.track, &mut overview);
    for (layout, &track) in layouts.iter_mut().zip(&tracks) {
        minimap.extend_timeline(loaded, track, layout);
    }
    let range = app.playback.repeat;
    let chords: Vec<_> = tracks
        .iter()
        .map(|&track| chord_timing::highlight(app, track))
        .collect();
    let state = &mut app.piano_roll;
    state
        .viewport
        .begin(crate::view::timeline_controls::Screen::PianoRoll);
    state.viewport.panes = tracks.len();
    let reveal = std::mem::take(&mut state.reveal);
    let mut out = Outcome::default();
    if let Some(error) = state.error.as_ref().or(minimap.error.as_ref()) {
        ui.colored_label(Color32::LIGHT_RED, error);
    }
    let area = state.viewport.bars(
        ui,
        state.drag.is_none()
            && state.range_drag.is_none()
            && state.repeat_drag.is_none()
            && state.edge_drag.is_none(),
    );
    let time_scale = overview.scale * state.viewport.zoom().x;
    let timeline = state.viewport.binding(
        &time_scale,
        (overview.size.x / overview.scale).round() as u64,
    );
    let (area, navigated) =
        state
            .viewport
            .minimap_on(ui, area, &overview, minimap.layers(), playing, &timeline);
    following.minimap(&mut app.playback.follow, navigated);
    let area = area.with_min_x(area.left() + keyboard::WIDTH);
    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
        ui.spacing_mut().scroll.floating = false;
        let notes_height =
            (area.height() - ui.spacing().scroll.allocated_width() - repeat::HEIGHT).max(1.0);
        let pane_height = notes_height / tracks.len() as f32;
        for layout in &mut layouts {
            if tracks.len() == 2 {
                panes::fit_height(layout, pane_height);
            }
            state.viewport.scale_layout(layout);
        }
        let mut scrollbar = egui::Rect::from_min_max(
            area.min,
            area.max - egui::Vec2::splat(ui.spacing().scroll.allocated_width()),
        )
        .shrink2(egui::Vec2::splat(crate::view::timeline_zoom::BUTTON));
        scrollbar.min.y += repeat::HEIGHT;
        let mut scroll = egui::ScrollArea::both()
            .id_salt(("piano-roll", tracks.clone()))
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .scroll_bar_rect(scrollbar)
            .drag_to_scroll(false);
        let pending = state.viewport.pending.take();
        if let Some(tick) = following.tick() {
            scroll = scroll.scroll_offset(egui::vec2(
                (tick as f32 * layouts[0].scale
                    - (area.width() - ui.spacing().scroll.allocated_width()) / 2.0)
                    .max(0.0),
                pending.unwrap_or(state.viewport.scroll).y,
            ));
        } else if reveal && !navigated {
            scroll = scroll.scroll_offset(egui::vec2(layouts[0].start, 0.0));
        } else if let Some(offset) = pending {
            scroll = scroll.scroll_offset(offset);
        }
        let mut keyboards = Vec::new();
        let output = scroll.show_viewport(ui, |ui, visible| {
            let width = layouts
                .iter()
                .map(|l| l.size.x)
                .fold(visible.width(), f32::max);
            let height = if tracks.len() == 2 {
                notes_height + (layouts[0].size.y - pane_height).max(0.0)
            } else {
                layouts[0].size.y.max(notes_height)
            };
            let (whole, _) = ui.allocate_exact_size(
                egui::vec2(width, height + repeat::HEIGHT),
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
            state.repeat_rect = Some(row.intersect(screen));
            out.repeat_action = timeline.repeat(ui, row, range, &mut state.repeat_drag);
            for (slot, (&track, layout)) in tracks.iter().zip(&layouts).enumerate() {
                let clip = egui::Rect::from_min_size(
                    egui::pos2(
                        screen.left(),
                        screen.top() + repeat::HEIGHT + slot as f32 * pane_height,
                    ),
                    egui::vec2(screen.width(), pane_height),
                );
                let rect = egui::Rect::from_min_size(
                    egui::pos2(whole.left(), clip.top() - visible.min.y),
                    egui::vec2(width, layout.size.y.max(pane_height)),
                );
                keyboards.push((
                    egui::Rect::from_min_size(
                        egui::pos2(screen.left() - keyboard::WIDTH, rect.top()),
                        egui::vec2(keyboard::WIDTH, layout.size.y),
                    ),
                    egui::Rect::from_min_max(
                        egui::pos2(screen.left() - keyboard::WIDTH, clip.top()),
                        egui::pos2(screen.left(), clip.bottom()),
                    ),
                ));
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.set_clip_rect(ui.clip_rect().intersect(clip));
                    let action = canvas::show(ui, rect, layout, state, slot, &chords[slot]);
                    out.trill_click = crate::view::meta_text::show(
                        ui,
                        loaded,
                        track,
                        &app.config,
                        rect.with_min_y(clip.top()),
                        layout.scale,
                    )
                    .or(out.trill_click);
                    if action.navigated {
                        app.playback.follow = false;
                    }
                    if let Some(tick) = action.audition {
                        out.audition = Some((slot, track, tick));
                    }
                    if let Some((on, part, shift)) = action.edit {
                        out.released = loaded.note_views[track].durations[on]
                            .map(|duration| (track, on, duration, part, shift));
                    }
                    if tracks.len() == 2 {
                        ui.painter().rect_stroke(
                            clip,
                            0.0,
                            egui::Stroke::new(
                                1.0_f32,
                                if slot == active_slot {
                                    crate::view::ACTIVE_PANE
                                } else {
                                    Color32::from_gray(75)
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
            }
            if let Some(tick) = playing {
                ui.painter().vline(
                    whole.left() + tick as f32 * layouts[0].scale,
                    screen.y_range(),
                    egui::Stroke::new(2.0_f32, repeat::PLAYHEAD),
                );
            }
        });
        let bounds = ui.clip_rect();
        for (slot, ((rect, clip), layout)) in keyboards.into_iter().zip(&layouts).enumerate() {
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(clip.intersect(bounds));
                if !keyboard_enabled {
                    ui.disable();
                }
                if let Some(pitch) =
                    keyboard::show(ui, rect, clip, layout, &mut state.keyboard, slot)
                {
                    out.key = Some((tracks[slot], pitch));
                }
            });
        }
        let panel = output
            .inner_rect
            .with_min_y(output.inner_rect.top() + repeat::HEIGHT);
        state.viewport.finish(ui, area, panel, output.state.offset);
    });
    Some(out)
}
