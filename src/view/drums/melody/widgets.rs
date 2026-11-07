use super::super::commands::Op;
use super::commands::{apply, Action};
use super::*;
use egui::{Color32, Rect, Stroke};

const LABEL: f32 = 44.0;
pub(super) const HEADER: f32 = 24.0;
pub(super) const CELL: egui::Vec2 = egui::vec2(22.0, 18.0);
const GAP: f32 = 3.0;
/// A note whose start or end is off the step grid (MML or recorded input).
const OFF_GRID: Color32 = Color32::from_rgb(230, 150, 60);
const HEAD: Stroke = Stroke {
    width: 2.0,
    color: Color32::from_gray(16),
};
/// How far an empty cell outside its step's harmony darkens.
const OUTSIDE: f32 = 0.6;

/// Left edge of `step`'s cell, from the grid's left; a gap opens every four steps.
pub(super) fn x(step: usize) -> f32 {
    LABEL + step as f32 * CELL.x + (step / 4 + 1) as f32 * GAP
}

pub(super) fn show(a: &mut App, ctx: &egui::Context) {
    if a.drum_editor
        .melody
        .gesture
        .as_ref()
        .is_some_and(|g| !g.valid(a))
    {
        a.drum_editor.melody.gesture = None;
    }
    let base = a.loaded.as_ref().map(|l| l.visible().clone());
    let track = a.track;
    let limit = crate::view::patterns::mode::editing(a, track)
        .and_then(|context| a.loaded.as_ref()?.edit_range(context).ok())
        .map_or(0, |(_, end)| end);
    let original = notes(a);
    let preview = a
        .drum_editor
        .melody
        .gesture
        .as_ref()
        .and_then(|g| g.music().ok());
    let measure = playback::measure(a);
    let Layout {
        rows,
        columns,
        tonic,
    } = layout(a);
    let shown_set = a.config.melody_rows(a.patterns.container);
    let notes = preview
        .as_ref()
        .map_or_else(|| original.clone(), |m| step_notes::notes(&m.tracks[track]));
    let lanes: Vec<Vec<Cell>> = measure.as_ref().map_or_else(
        |_| Vec::new(),
        |m| rows.iter().map(|&p| cells(a, &notes, m, p)).collect(),
    );
    let sounding = measure.as_ref().ok().and_then(|m| playback::sounding(a, m));
    let soloable = playback::soloable(a);
    let (cursor_pitch, cursor_step) = (pitch(a), step(a));
    // The rows of the highest and lowest shown notes; rows, not pitches, so a filtered set centers too.
    let spread = notes
        .iter()
        .filter_map(|n| rows.iter().position(|&p| p == n.pitch))
        .fold(None, |s: Option<(usize, usize)>, r| {
            Some(s.map_or((r, r), |(top, bottom)| (top.min(r), bottom.max(r))))
        });
    let container = a.patterns.container;
    let zoom = a.config.melody_zoom(container);
    let mut zoomed = None;
    let row_h = CELL.y * zoom;
    let mut surface = super::super::surface::Surface::new(a, ctx, sounding).playing_line();
    let mut action = None;
    let mut finished = None;
    let mut open = true;
    super::super::window::Kind::Melody.show(a, ctx, &mut open, |a, ui| {
        let state = &mut a.drum_editor;
        ui.horizontal_wrapped(|ui| {
            if ui.button("◀").on_hover_text("PageUp").clicked() {
                action = Some(Action::Common(Op::Page(false)));
            }
            ui.label((state.page + 1).to_string());
            if ui.button("▶").on_hover_text("PageDown").clicked() {
                action = Some(Action::Common(Op::Page(true)));
            }
            kit::controls::boundaries(ui, state, |op| action = Some(Action::Common(op)));
            if ui
                .selectable_label(state.loop_enabled, "Loop")
                .on_hover_text("R")
                .clicked()
            {
                action = Some(Action::Common(Op::Loop));
            }
            if ui
                .add_enabled(
                    soloable,
                    egui::SelectableLabel::new(!state.all_tracks, "Solo"),
                )
                .on_hover_text("S")
                .clicked()
            {
                action = Some(Action::Common(Op::Scope));
            }
            if let Some(set) = super::controls::row_sets(ui, shown_set) {
                action = Some(Action::Rows(Some(set)));
            }
        });
        match &measure {
            Ok(m) => {
                let steps = m.steps.len();
                ui.horizontal_top(|ui| {
                    // A zoom's offset waits for the frame that draws rows at the new height.
                    let applied = state.melody.pending.take();
                    if let Some(y) = applied {
                        state.melody.scroll = y;
                    }
                    zoomed = super::controls::zoom_bar(ui, &mut state.melody, zoom);
                    let mut area = egui::ScrollArea::both()
                        .drag_to_scroll(false)
                        .animated(false)
                        .auto_shrink([false, false])
                        .max_height(ui.available_height());
                    if let Some(y) = applied {
                        area = area.vertical_scroll_offset(y);
                    }
                    let scroll = area.show(ui, |ui| {
                        let size = egui::vec2(x(steps), HEADER + rows.len() as f32 * row_h);
                        let (rect, response) =
                            ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                        let clip = ui.clip_rect();
                        if let Some(base) = &base {
                            finished = gesture::Grid {
                                rect,
                                clip,
                                row_h,
                                rows: &rows,
                                measure: m,
                                notes: &original,
                                base,
                                track,
                                limit,
                            }
                            .interact(ui, &response, &mut state.melody);
                            if let Some(gesture) =
                                state.melody.gesture.as_ref().or(finished.as_ref())
                            {
                                let (pitch, step) = gesture.cursor();
                                state.melody.pitch = Some(pitch);
                                state.field = step + 2;
                            }
                        }
                        let cell = |row: usize, step: usize| {
                            let top = rect.top() + HEADER + row as f32 * row_h;
                            Rect::from_min_max(
                                egui::pos2(rect.left() + x(step) + 1.0, top + 1.0),
                                egui::pos2(rect.left() + x(step) + CELL.x - 1.0, top + row_h - 1.0),
                            )
                        };
                        let grid = Rect::from_min_max(
                            egui::pos2(rect.left() + LABEL, rect.top()),
                            rect.max,
                        );
                        let painter = ui.painter().clone();
                        for (row, (&pitch, lane)) in rows.iter().zip(&lanes).enumerate() {
                            let band = cell(row, 0);
                            if band.bottom() < clip.top() || band.top() > clip.bottom() {
                                continue;
                            }
                            let label = Rect::from_min_max(
                                egui::pos2(rect.left(), band.top()),
                                egui::pos2(rect.left() + LABEL - GAP, band.bottom()),
                            );
                            let empty = super::super::surface::step_fill(ui, false, false);
                            let empty = if black(pitch) {
                                empty.lerp_to_gamma(Color32::BLACK, 0.35)
                            } else {
                                empty
                            };
                            painter.rect_filled(label, 2.0, empty);
                            painter.text(
                                label.left_center() + egui::vec2(4.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                name(pitch),
                                egui::FontId::proportional(12.0 * (row_h / CELL.y).min(1.0)),
                                ui.visuals().text_color(),
                            );
                            for (step, kind) in lane.iter().enumerate() {
                                let mut r = cell(row, step);
                                let outside = columns.get(step).is_some_and(|c| !c.contains(pitch));
                                let fill = match kind {
                                    Cell::Empty if outside => {
                                        empty.lerp_to_gamma(Color32::BLACK, OUTSIDE)
                                    }
                                    Cell::Empty => empty,
                                    Cell::Head { aligned: true }
                                    | Cell::Continuation { aligned: true } => {
                                        super::super::surface::step_fill(ui, true, false)
                                    }
                                    _ => OFF_GRID,
                                };
                                if matches!(kind, Cell::Continuation { .. }) && step > 0 {
                                    // One note is one unbroken band across the cells it covers.
                                    r.min.x = cell(row, step - 1).right();
                                }
                                painter.rect_filled(r, 0.0, fill);
                                if matches!(kind, Cell::Head { .. }) {
                                    painter.line_segment([r.left_top(), r.left_bottom()], HEAD);
                                }
                                let selected = pitch == cursor_pitch && step == cursor_step;
                                if selected {
                                    let c = cell(row, step);
                                    painter.rect_stroke(
                                        c,
                                        2.0,
                                        surface.cursor,
                                        egui::StrokeKind::Inside,
                                    );
                                    surface.selection(ui, c, true);
                                }
                                #[cfg(test)]
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new(("melody-cell", pitch, step)), r);
                                    d.insert_temp(
                                        egui::Id::new(("melody-outside", pitch, step)),
                                        outside,
                                    );
                                });
                            }
                            if pitch % 12 == tonic {
                                // Each octave of rows starts at the tonic; the line is its floor.
                                let y = band.bottom() + 0.5;
                                painter.line_segment(
                                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                                    Stroke::new(1.5_f32, ui.visuals().strong_text_color()),
                                );
                                #[cfg(test)]
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new(("melody-octave", pitch)), y)
                                });
                            }
                        }
                        for step in 0..steps {
                            let column = Rect::from_min_max(
                                egui::pos2(rect.left() + x(step), rect.top() + HEADER),
                                egui::pos2(rect.left() + x(step) + CELL.x, rect.bottom()),
                            );
                            surface.cell(column, step);
                            let header = Rect::from_min_size(
                                egui::pos2(column.left(), rect.top()),
                                egui::vec2(CELL.x, HEADER),
                            );
                            super::super::surface::column(
                                ui,
                                step,
                                header,
                                CELL.x,
                                column.left() - GAP / 2.0,
                                grid,
                            );
                        }
                        surface.finish(ui);
                        if let Some(row) = rows.iter().position(|&p| p == cursor_pitch)
                            && std::mem::take(&mut state.melody.reveal)
                        {
                            let mut target = cell(row, cursor_step.min(steps.saturating_sub(1)));
                            let center = std::mem::take(&mut state.melody.center);
                            if center && let Some((top, bottom)) = spread {
                                target.min.y = cell(top, 0).top();
                                target.max.y = cell(bottom, 0).bottom();
                            }
                            ui.scroll_to_rect(
                                target.expand(2.0),
                                center.then_some(egui::Align::Center),
                            );
                        }
                        if response.clicked()
                            && let Some(at) = response.interact_pointer_pos()
                        {
                            let row = ((at.y - rect.top() - HEADER) / row_h).floor();
                            let step = (0..steps).find(|&s| {
                                let c = cell(0, s);
                                (c.left() - 1.0..=c.right() + 1.0).contains(&at.x)
                            });
                            let pitch = (row >= 0.0).then(|| rows.get(row as usize)).flatten();
                            if let Some(&pitch) = pitch {
                                if at.x < rect.left() + LABEL - GAP {
                                    state.melody.pitch = Some(pitch);
                                    action = Some(Action::Listen(pitch));
                                } else if let Some(step) = step {
                                    state.melody.pitch = Some(pitch);
                                    state.field = step + 2;
                                    action = Some(Action::Toggle);
                                }
                            }
                        }
                    });
                    state.melody.scroll = scroll.state.offset.y;
                    #[cfg(test)]
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::new("melody-clip"), scroll.inner_rect)
                    });
                });
            }
            Err(error) => {
                ui.colored_label(Color32::LIGHT_RED, error);
            }
        }
        if let Some(error) = &state.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    });
    if let (Some(zoom), Some(container)) = (zoomed, container) {
        a.config.remember_melody_zoom(container, zoom);
    }
    if !open {
        a.drum_editor.melody.gesture = None;
        finished = None;
        action = Some(Action::Common(Op::Cancel));
    }
    if let Some(gesture) = finished {
        gesture.commit(a);
    }
    if let Some(action) = action {
        apply(a, action);
    }
    if a.drum_editor.kit.preview.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(35));
    }
}
