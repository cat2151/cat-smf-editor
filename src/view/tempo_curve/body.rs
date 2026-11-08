//! The tempo curve inside whatever holds it: its own window or a pane.
use super::*;
use cat_smf_editor::model::Music;
use std::sync::Arc;

/// What one frame of the curve asks the app to do once drawing is over.
#[derive(Default)]
pub(super) struct Outcome {
    pub repeat_action: Option<repeat::Action>,
    pub audition_tick: Option<u64>,
    /// The curve to store after this frame; `Some(None)` removes it.
    pub edited: Option<Option<Curve>>,
    pub reference_edit: Option<cat_smf_editor::edit::reference_tempo::Reference>,
    pub trill_click: Option<(usize, usize)>,
}

/// Draws the curve into `ui` from the document `prepared` for display.
pub(super) fn show(app: &mut App, ui: &mut egui::Ui, prepared: &Arc<Music>) -> Outcome {
    let ctx = ui.ctx().clone();
    let mut out = Outcome::default();
    let Some(loaded) = &app.loaded else {
        return out;
    };
    let reference_points = cat_smf_editor::edit::reference_tempo::read(prepared);
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
    let mut following = Follow::new(&ctx, app.playback.follow, playing);
    let range = app.playback.repeat;
    let state = &mut app.tempo_curve;
    let reveal = std::mem::take(&mut state.reveal);
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
        state.drag.is_none() && state.repeat_drag.is_none() && state.references.axis().is_none(),
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
        let scrollbar =
            egui::Rect::from_min_size(area.min, egui::vec2(ui.available_width(), area.height()));
        let mut scroll = egui::ScrollArea::horizontal()
            .id_salt("tempo-curve")
            .scroll_bar_rect(scrollbar.shrink2(egui::vec2(crate::view::timeline_zoom::BUTTON, 0.0)))
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
        if let Some(tick) = following.tick() {
            state.viewport.pending_scroll = None;
            let width = (ui.available_width() - 2.0 * HIT).max(0.0);
            scroll = scroll
                .horizontal_scroll_offset((tick as f32 * layout.scale - width / 2.0).max(0.0));
        } else if reveal && !navigated {
            scroll = scroll.horizontal_scroll_offset(layout.start);
        } else if let Some(offset) = state.viewport.pending_scroll.take() {
            scroll = scroll.horizontal_scroll_offset(offset);
        }
        let output = scroll.show_viewport(ui, |ui, viewport| {
            let mut size = size(&layout, curve.as_ref(), viewport.height() - repeat::HEIGHT);
            size.x = size.x.max(viewport.width());
            if let Some(last) = references.last() {
                size.x = size.x.max(last.tick as f32 * layout.scale + PX_PER_QUARTER);
            }
            let (whole, _) = ui
                .allocate_exact_size(size + egui::vec2(0.0, repeat::HEIGHT), egui::Sense::hover());
            let row = egui::Rect::from_min_size(whole.min, egui::vec2(size.x, repeat::HEIGHT));
            out.repeat_action = timeline.repeat(
                ui,
                row.shrink2(egui::vec2(HIT, 0.0)),
                range,
                &mut state.repeat_drag,
            );
            let panels =
                egui::Rect::from_min_max(whole.min + egui::vec2(0.0, repeat::HEIGHT), whole.max)
                    .shrink2(egui::vec2(HIT, HIT));
            let middle = panels.center().y;
            let notes_rect =
                egui::Rect::from_min_max(panels.min, egui::pos2(panels.right(), middle - 4.0));
            let rect =
                egui::Rect::from_min_max(egui::pos2(panels.left(), middle + 4.0), panels.max);
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
            out.reference_edit = changed;
            let (preview, hover, changed) =
                interaction::update(state, &roll, &response, curve.as_ref(), reference_handled);
            out.edited = changed;
            out.audition_tick = state.viewport.notes(ui, notes_rect, layout.scale);
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
                out.trill_click = crate::view::meta_text::show_music(
                    ui,
                    prepared,
                    &app.config,
                    panel,
                    layout.scale,
                )
                .or(out.trill_click);
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
    out
}
