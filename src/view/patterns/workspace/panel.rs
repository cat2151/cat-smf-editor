use super::*;
mod candidates;
mod drawing;
mod playhead;
mod scrolling;
use drawing::{cell, measure_in_block, paint_part, place_beacon, trash, Block};
/// A placement being dragged: its row and first measure.
#[derive(Clone, Copy)]
struct Placed(usize, u64);
pub fn button(
    ui: &mut egui::Ui,
    p: Pane,
    id: &'static str,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let r = ui.button(label);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), id)), r.rect));
    let _ = (p, id);
    r
}
pub fn show(a: &mut App, ui: &mut egui::Ui, p: Pane) {
    if p == Pane::Song {
        egui::ScrollArea::vertical()
            .id_salt("song-body")
            .drag_to_scroll(false)
            .auto_shrink([false, false])
            .show(ui, |ui| body(a, ui, p));
    } else {
        body(a, ui, p);
    }
}

fn body(a: &mut App, ui: &mut egui::Ui, p: Pane) {
    if input_owner(a, ui.ctx()) || crate::view::modal::is_open(a) {
        ui.disable();
    }
    let Some(l) = &a.loaded else { return };
    let Some((_, c)) = &l.arrangement else {
        let e = Catalog::read(&l.music).err().unwrap_or_default();
        ui.colored_label(ui.visuals().error_fg_color, e);
        return;
    };
    let c = c.clone();
    if p == Pane::Scene
        && !c
            .scenes
            .iter()
            .any(|s| Some(s.id) == a.patterns.workspace.scene)
    {
        a.patterns.workspace.scene = c.scenes.first().map(|s| s.id);
    }
    let id = scene_id(a, p);
    let count = if p == Pane::Scene && id.is_none() {
        0
    } else {
        rows(&c, id)
    };
    let s = a.patterns.workspace.selection_mut(p);
    let events = ui.input(|i| i.events.clone());
    let ime = s.composing || events.iter().any(|e| matches!(e, egui::Event::Ime(_)));
    for e in &events {
        if let egui::Event::Ime(e) = e {
            match e {
                egui::ImeEvent::Preedit(t) => s.composing = !t.is_empty(),
                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => s.composing = false,
                _ => {}
            }
        }
    }
    let was_focused = focused_query(ui.ctx()) == Some(p);
    s.row = s.row.min(count.saturating_sub(1));
    let length = id
        .and_then(|id| c.scenes.iter().find(|s| s.id == id))
        .map(|s| s.meters.len() as u64);
    if let Some(len) = length {
        s.measure = s.measure.min(len).max(1);
    }

    // The Scene window runs this body too; its time is not Song's.
    let song_span = |name| (p == Pane::Song).then(|| crate::frame_trace::span(name));
    let minimap_span = song_span("song/ui/minimap");
    super::minimap::show(a, ui, p);
    drop(minimap_span);
    ui.horizontal(|ui| {
        if p == Pane::Scene {
            let mut selected = id;
            egui::ComboBox::from_id_salt("active-scene")
                .selected_text(
                    c.scenes
                        .iter()
                        .find(|s| Some(s.id) == id)
                        .map_or("Scene", |s| s.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    for s in &c.scenes {
                        ui.selectable_value(&mut selected, Some(s.id), &s.name);
                    }
                });
            if selected != id
                && let Some(id) = selected
            {
                let r = open_scene(a, id);
                result(a, r);
            }
            if button(ui, p, "create", "New scene").clicked() {
                open_dialog(a, ui.ctx(), dialog::Purpose::Create, p);
            }
        }
        // Same states as the main transport bar.
        let playing =
            crate::play::transport::preparing(a) || crate::play::transport::tick(a).is_some();
        let label = if playing {
            egui::RichText::new("▶ Playing").color(egui::Color32::LIGHT_GREEN)
        } else {
            egui::RichText::new("Stopped")
        };
        if button(ui, p, "transport", label).clicked() {
            if playing {
                crate::play::transport::stop(a);
            } else {
                play(a, p);
            }
        }
        if p == Pane::Song {
            ui.checkbox(&mut a.patterns.workspace.scene_window, "Scene");
            if a.patterns.recording.minimized()
                && button(ui, p, "keyboard", "🎹")
                    .on_hover_text("Virtual MIDI keyboard (Shift+R)")
                    .clicked()
            {
                crate::view::recording::restore(ui.ctx());
            }
        }
    });
    if let Some(e) = &a.patterns.workspace.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    let l = a.loaded.as_ref().unwrap();
    let Some((catalog, c)) = &l.arrangement else {
        return;
    };
    let c = c.clone();
    let id = scene_id(a, p);
    let count = if p == Pane::Scene && id.is_none() {
        0
    } else {
        rows(&c, id)
    };
    let s = a.patterns.workspace.selection(p);
    let zoom = if p == Pane::Song {
        a.patterns.workspace.timeline.zoom()
    } else {
        1.0
    };
    let cell_width = super::timeline::CELL * zoom;
    let columns = ((ui.available_width() / cell_width).ceil() as u64).max(12);
    let end = length.map_or_else(
        || {
            c.song
                .iter()
                .map(|v| {
                    v.measure
                        .saturating_add(model::length(&c, v.part).unwrap_or(1))
                })
                .max()
                .unwrap_or(1)
                .max(
                    cat_smf_editor::score::musical_time::MusicalTime::new(&l.music)
                        .map(0)
                        .map_or(1, |map| map.position(c.song_length).measure as u64 + 1),
                )
                .max(s.measure.saturating_add(columns))
        },
        |n| n.saturating_add(1),
    );
    let position = (s.row, s.measure);
    let selected = s.cells(p).then_some((s.row, s.measure));
    let choices = choices(a, p);
    let (music, keys) = (&l.music, &l.keys);
    let lanes = lanes(&c, id);
    let beacon = (p == Pane::Song)
        .then(|| super::super::beacon::step(a))
        .flatten();
    let mut first_cell = None;
    let mut clicked = None;
    let mut dropped = None;
    let mut moved = None;
    let mut commented = None;
    let mut scrolling = scrolling::Matrix::new(ui, p, position, end, cell_width);
    let head = playhead::x(a, &c, p, cell_width, scrolling.label());
    let timeline = if p == Pane::Song {
        &mut a.patterns.workspace.timeline
    } else {
        &mut a.patterns.workspace.scene_timeline
    };
    let pending = timeline.viewport.pending.take();
    scrolling.turn_page(ui, head, pending.is_none());
    if let Some(offset) = pending {
        // The viewport scales the whole offset, but the row label before measure one keeps its width.
        let scale = timeline
            .cells
            .map_or(1.0, |(stride, _)| cell_width / stride);
        scrolling.offset = (offset.x + scrolling.label() * (1.0 - scale)).max(0.0);
    }
    let mut repeat_action = None;
    let matrix_span = song_span("song/ui/matrix");
    let output = egui::ScrollArea::both()
        .drag_to_scroll(false)
        .id_salt((p.title(), "matrix"))
        .max_height(160.0)
        .horizontal_scroll_offset(scrolling.offset)
        .show_viewport(ui, |ui, viewport| {
            let origin = ui.max_rect().left();
            let (area, first, end) = scrolling.visible(ui, viewport);
            if p == Pane::Song {
                repeat_action = scrolling.repeat(
                    ui,
                    music,
                    &mut a.patterns.workspace.timeline,
                    a.playback.repeat,
                );
            }
            let mut area = area;
            if p == Pane::Song {
                area.min.y = ui.cursor().top();
            }
            let cells = ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                egui::Grid::new((p.title(), "cells"))
                    .min_col_width(0.0)
                    .spacing(egui::vec2(0.0, ui.spacing().item_spacing.y))
                    .show(ui, |ui| {
                        scrolling.row_label(ui, "");
                        for m in first..end {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(
                                    cell_width,
                                    ui.text_style_height(&egui::TextStyle::Body),
                                ),
                                egui::Sense::hover(),
                            );
                            ui.painter_at(rect).text(
                                rect.left_top(),
                                egui::Align2::LEFT_TOP,
                                m.to_string(),
                                egui::TextStyle::Body.resolve(ui.style()),
                                ui.visuals().text_color(),
                            );
                        }
                        ui.end_row();
                        for row in 0..count {
                            let lane = lanes.get(row).copied();
                            scrolling.row_label(ui, &(row + 1).to_string());
                            let mut block: Option<Block> = None;
                            for measure in first..end {
                                let covering =
                                    lane.and_then(|l| model::covering(&c, id, l, measure));
                                let continuation = covering
                                    .filter(|(_, start)| *start < measure)
                                    .map(|(_, start)| start);
                                let (rect, r) = ui.allocate_exact_size(
                                    egui::vec2(cell_width, 30.0),
                                    egui::Sense::click_and_drag(),
                                );
                                if let Some((_, start)) = covering {
                                    r.dnd_set_drag_payload(Placed(row, start));
                                }
                                let target = r.dnd_hover_payload::<PatternOrScene>().is_some()
                                    || r.dnd_hover_payload::<Placed>().is_some();
                                let cursor = selected == Some((row, measure));
                                if block.as_ref().map(|b| b.start)
                                    != covering.map(|(_, start)| start)
                                    && let Some(b) = block.take()
                                {
                                    b.finish(ui);
                                }
                                if let Some((part, start)) = covering {
                                    let faded = lane
                                        .is_some_and(|l| model::commented_out(&c, id, l, measure));
                                    let b = block.get_or_insert_with(|| {
                                        let label = choices
                                            .iter()
                                            .find(|(p, _)| *p == part)
                                            .map_or("", |(_, s)| s.as_str());
                                        Block::begin(ui, start, rect, label, faded)
                                    });
                                    b.extend(rect, cursor);
                                    let k = (measure - start) as usize;
                                    measure_in_block(
                                        ui,
                                        rect,
                                        cursor,
                                        target,
                                        faded,
                                        |painter, area| {
                                            paint_part(
                                                painter,
                                                area,
                                                music,
                                                keys,
                                                catalog,
                                                &c,
                                                part,
                                                Some((k, k + 1)),
                                            );
                                        },
                                    );
                                } else {
                                    cell(ui, rect, "·", cursor || target, false, |_, _| {});
                                }
                                if selected == Some((row, measure))
                                    || selected.is_none() && (row, measure) == (0, 1)
                                {
                                    first_cell = Some(rect);
                                }
                                #[cfg(test)]
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(
                                        egui::Id::new((p.title(), "cell", row, measure)),
                                        r.rect,
                                    )
                                });
                                // Taking a payload clears it whatever its type, so ask first.
                                if r.dnd_hover_payload::<Placed>().is_some()
                                    && let Some(from) = r.dnd_release_payload::<Placed>()
                                {
                                    moved = Some((*from, row, measure));
                                } else if let Some(part) = r.dnd_release_payload::<PatternOrScene>()
                                {
                                    dropped = Some((row, measure, *part));
                                } else if r.clicked() {
                                    clicked = Some((row, measure, continuation.unwrap_or(measure)));
                                } else if r.secondary_clicked() && covering.is_some() {
                                    commented = Some((row, measure));
                                }
                            }
                            if let Some(b) = block {
                                b.finish(ui);
                            }
                            ui.end_row();
                        }
                    });
            });
            if let Some(x) = head {
                playhead::paint(ui, origin + x, cells.response.rect);
            }
        });
    drop(matrix_span);
    scrolling.finish(ui, p, &output);
    let timeline = if p == Pane::Song {
        &mut a.patterns.workspace.timeline
    } else {
        &mut a.patterns.workspace.scene_timeline
    };
    scrolling.finish_timeline(&output, timeline);
    if let Some(action) = repeat_action {
        super::timeline::apply(a, action);
    }
    if let Some((row, from, measure)) = clicked {
        let s = a.patterns.workspace.selection_mut(p);
        s.placing = true;
        s.row = row;
        s.measure = measure;
        click_body(a, p);
        // Cell click explicitly auditions the arrangement; displaying a body never does.
        crate::play::arrangement::start_at(a, id, from);
    }
    if let Some((row, measure)) = commented {
        let s = a.patterns.workspace.selection_mut(p);
        s.placing = true;
        s.row = row;
        s.measure = measure;
        super::commands::comment_out(a, p);
    }
    if let Some((Placed(from_row, start), row, measure)) = moved {
        let lanes = lanes.clone();
        let r = mutate(a, |m| {
            let to = materialize(m, id, row)?;
            model::move_part(m, id, (lanes[from_row], start), (to, measure))
        });
        if r.is_ok() {
            let s = a.patterns.workspace.selection_mut(p);
            s.placing = true;
            s.row = row;
            s.measure = measure;
        }
        result(a, r);
    }
    if let Some((row, measure, part)) = dropped {
        let s = a.patterns.workspace.selection_mut(p);
        s.placing = true;
        s.row = row;
        s.measure = measure;
        s.choice = Some(part);
        let r = place(a, p, Some(part));
        result(a, r);
    }
    let mut deleted = false;
    ui.horizontal(|ui| match trash(ui, p) {
        Some(drawing::Discard::Placement(Placed(row, start))) => {
            let r = mutate(a, |m| model::place(m, id, lanes[row], start, None));
            result(a, r);
        }
        Some(drawing::Discard::Pattern(id)) => {
            let r = super::deletion::pattern(a, id);
            result(a, r);
            deleted = true;
        }
        None => {}
    });
    if deleted {
        return;
    }
    let _candidates_span = song_span("song/ui/candidates");
    candidates::show(a, ui, p, &c, first_cell, beacon, (ime, was_focused));
}
