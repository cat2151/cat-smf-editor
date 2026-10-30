use super::*;

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.tempo_curve.open {
        app.tempo_curve.viewport.suspend();
        app.tempo_curve.repeat_drag = Default::default();
        app.tempo_curve.drag = None;
        app.tempo_curve.draft = None;
        return;
    }
    if app.loaded.is_none() {
        return;
    }
    let prepared = match trills::prepare(app) {
        Ok(music) => music,
        Err(error) => {
            app.tempo_curve.error = Some(error);
            std::sync::Arc::new(app.loaded.as_ref().unwrap().music.clone())
        }
    };
    let Some(loaded) = &app.loaded else {
        return;
    };
    let reference_points = cat_smf_editor::edit::reference_tempo::read(&prepared);
    let tracks: Vec<usize> = if loaded.music.tracks[app.track]
        .events
        .iter()
        .any(cat_smf_editor::score::note_view::note_on)
    {
        vec![app.track]
    } else {
        (0..loaded.music.tracks.len()).collect()
    };
    let mut layout = layout_of(loaded, &tracks, 0, crate::play::playback::location(app).0);
    let (curve, read_error) = match tempo_curve::read(&loaded.music) {
        Ok(curve) => (curve, None),
        Err(error) => (None, Some(error)),
    };
    let tempos = tempo_events(loaded);
    let playing = crate::play::transport::tick_for(app, 0);
    let mut following = Follow::new(ctx, app.playback.follow, playing);
    let range = app.playback.repeat;
    let mut repeat_action = None;
    let mut audition_tick = None;
    let state = &mut app.tempo_curve;
    let reveal = std::mem::take(&mut state.reveal);
    // The curve to store after this frame; `Some(None)` removes it.
    let mut edited: Option<Option<Curve>> = None;
    let mut reference_edit = None;
    let mut trill_click = None;
    let mut open = true;
    window::window(ctx, app.config.tempo_curve_maximized)
        .open(&mut open)
        .show(ctx, |ui| {
            if let Some(error) = state
                .error
                .as_ref()
                .or(read_error.as_ref())
                .or(reference_points.as_ref().err())
            {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
            let references = reference_points.as_deref().unwrap_or(&[]);
            let base_axis = Axis::new(
                tempos
                    .iter()
                    .map(|t| t.1)
                    .chain(references.iter().map(|p| p.bpm))
                    .chain(
                        curve
                            .iter()
                            .flat_map(|c| c.points.iter().chain(&c.controls))
                            .map(|p| p.bpm),
                    ),
            );
            state
                .viewport
                .begin(crate::view::timeline_controls::Screen::Tempo);
            let area = state.viewport.bars(
                ui,
                base_axis,
                state.drag.is_none()
                    && state.repeat_drag.is_none()
                    && state.references.axis().is_none(),
            );
            let end = references
                .last()
                .map_or(0.0, |p| p.tick as f32 * layout.scale + PX_PER_QUARTER)
                .max(size(&layout, curve.as_ref(), 0.0).x);
            let time_scale = layout.scale * state.viewport.time_zoom();
            let timeline = state
                .viewport
                .binding(&time_scale, (end / layout.scale).round() as u64);
            let (area, navigated) = state.viewport.minimap(
                ui,
                area,
                &layout,
                &timeline,
                viewport::minimap_tempos(&tempos, curve.as_ref(), references),
                playing,
            );
            following.minimap(&mut app.playback.follow, navigated);
            state.viewport.scale_layout(&mut layout);
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                // Reserve scrollbar space so it cannot intercept the lowest BPM points.
                ui.spacing_mut().scroll.floating = false;
                ui.set_max_width((area.width() - 16.0).max(1.0));
                let scrollbar = egui::Rect::from_min_size(
                    area.min,
                    egui::vec2(ui.available_width(), area.height()),
                );
                let mut scroll = egui::ScrollArea::horizontal()
                    .id_salt("tempo-curve")
                    .scroll_bar_rect(
                        scrollbar.shrink2(egui::vec2(crate::view::timeline_zoom::BUTTON, 0.0)),
                    )
                    .auto_shrink([false, false])
                    .drag_to_scroll(false)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
                if let Some(tick) = following.tick() {
                    state.viewport.pending_scroll = None;
                    let width = (ui.available_width() - 2.0 * HIT).max(0.0);
                    scroll = scroll.horizontal_scroll_offset(
                        (tick as f32 * layout.scale - width / 2.0).max(0.0),
                    );
                } else if reveal && !navigated {
                    scroll = scroll.horizontal_scroll_offset(layout.start);
                } else if let Some(offset) = state.viewport.pending_scroll.take() {
                    scroll = scroll.horizontal_scroll_offset(offset);
                }
                let output = scroll.show_viewport(ui, |ui, viewport| {
                    let mut size =
                        size(&layout, curve.as_ref(), viewport.height() - repeat::HEIGHT);
                    size.x = size.x.max(viewport.width());
                    if let Some(last) = references.last() {
                        size.x = size.x.max(last.tick as f32 * layout.scale + PX_PER_QUARTER);
                    }
                    let (whole, _) = ui.allocate_exact_size(
                        size + egui::vec2(0.0, repeat::HEIGHT),
                        egui::Sense::hover(),
                    );
                    let row =
                        egui::Rect::from_min_size(whole.min, egui::vec2(size.x, repeat::HEIGHT));
                    repeat_action = timeline.repeat(
                        ui,
                        row.shrink2(egui::vec2(HIT, 0.0)),
                        range,
                        &mut state.repeat_drag,
                    );
                    let panels = egui::Rect::from_min_max(
                        whole.min + egui::vec2(0.0, repeat::HEIGHT),
                        whole.max,
                    )
                    .shrink2(egui::vec2(HIT, HIT));
                    let middle = panels.center().y;
                    let notes_rect = egui::Rect::from_min_max(
                        panels.min,
                        egui::pos2(panels.right(), middle - 4.0),
                    );
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(panels.left(), middle + 4.0),
                        panels.max,
                    );
                    // A fixed id keeps the drag alive when the error line above appears or goes.
                    let response = ui.interact(
                        rect,
                        crate::view::command_input::preview_id(
                            ui.ctx(),
                            egui::Id::new("tempo-curve-points"),
                        ),
                        egui::Sense::click_and_drag(),
                    );
                    let axis = state.references.axis().unwrap_or_else(|| {
                        state
                            .drag
                            .map_or(state.viewport.axis.unwrap_or(base_axis), |d| d.axis)
                    });
                    let roll = Roll {
                        rect,
                        scale: layout.scale,
                        axis,
                    };
                    let (reference_preview, changed, reference_handled) =
                        state
                            .references
                            .update(&roll, &response, curve.as_ref(), references);
                    reference_edit = changed;
                    let (preview, hover, changed) = interaction::update(
                        state,
                        &roll,
                        &response,
                        curve.as_ref(),
                        reference_handled,
                    );
                    edited = changed;
                    audition_tick = state.viewport.notes(ui, notes_rect, layout.scale);
                    state.viewport.tempo_rect = Some(rect);
                    let previewed: Option<Vec<(u64, f64)>> = preview.as_ref().map(|p| {
                        tempo_curve::tempos(&loaded.music, p)
                            .into_iter()
                            .map(|(tick, micros)| (tick, 60_000_000.0 / f64::from(micros)))
                            .collect()
                    });
                    drawing::draw(
                        ui.painter(),
                        &roll,
                        notes_rect,
                        &layout,
                        preview.as_ref().or(curve.as_ref()),
                        previewed.as_ref().unwrap_or(&tempos),
                        hover,
                    );
                    note_guides::draw(ui, &roll, loaded);
                    references::draw(
                        ui.painter(),
                        &roll,
                        &reference_preview,
                        state.references.active(),
                    );
                    for panel in [notes_rect, rect] {
                        trill_click = crate::view::meta_text::show_music(
                            ui,
                            &prepared,
                            &app.config,
                            panel,
                            layout.scale,
                        )
                        .or(trill_click);
                    }
                    if let Some(tick) = playing {
                        let x = rect.left() + tick as f32 * layout.scale;
                        ui.painter().vline(
                            x,
                            whole.y_range(),
                            egui::Stroke::new(2.0_f32, repeat::PLAYHEAD),
                        );
                    }
                });
                if let Some(mut panel) = state.viewport.tempo_rect {
                    panel.min.x = output.inner_rect.left() + HIT;
                    panel.max.x = output.inner_rect.right() - HIT;
                    state.viewport.vertical_bar(ui, panel, base_axis);
                }
                state.viewport.horizontal.finish(
                    output.inner_rect.shrink2(egui::vec2(HIT, 0.0)),
                    output.state.offset.x,
                );
                state.viewport.horizontal_pages(
                    ui,
                    egui::Rect::from_min_max(
                        output.inner_rect.left_bottom(),
                        egui::pos2(output.inner_rect.right(), area.bottom()),
                    ),
                );
            });
        });
    if let Some(tick) = app.tempo_curve.viewport.seek.take() {
        crate::view::timeline_domain::click(
            crate::view::timeline_controls::Screen::Tempo,
            app,
            0,
            tick,
        );
    }
    if trill_click.is_some() {
        audition_tick = None;
        edited = None;
        reference_edit = None;
    }
    if let Some(tick) = audition_tick {
        interaction::audition(app, tick);
    }
    if let Some(action) = repeat_action {
        repeat::apply(app, action);
    }
    if !open {
        app.tempo_curve.open = false;
    }
    if let Some(point) = reference_edit {
        app.tempo_curve.error = app
            .edit_music(|music| {
                *music = (*prepared).clone();
                cat_smf_editor::edit::reference_tempo::set(music, point.tick, point.bpm)
            })
            .err();
    }
    if let Some(curve) = edited {
        let result = app.edit_music(|music| {
            *music = (*prepared).clone();
            tempo_curve::write(music, curve.as_ref())
        });
        app.tempo_curve.error = result.err();
    }
    if let Some((track, event)) = trill_click {
        app.tempo_curve.error =
            crate::view::trill_edit::begin_prepared(app, track, event, &prepared).err();
    }
}
