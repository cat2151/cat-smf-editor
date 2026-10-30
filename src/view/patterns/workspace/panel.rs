use super::*;
mod candidates;
mod drawing;
mod scrolling;
use drawing::{cell, paint_part, place_beacon, trash};
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
    let c = match scene::read(&l.music).and_then(|c| Catalog::read(&l.music).map(|_| c)) {
        Ok(c) => c,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
            return;
        }
    };
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

    super::minimap::show(a, ui, p);
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
        }
    });
    if let Some(e) = &a.patterns.workspace.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    let c = scene::read(&a.loaded.as_ref().unwrap().music).unwrap_or(c);
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
    let cell_width = (90.0 * zoom).max(2.0);
    let gap = ui.spacing().item_spacing.x * zoom.min(1.0);
    let columns = ((ui.available_width() / (cell_width + gap)).ceil() as u64).max(12);
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
                    cat_smf_editor::score::musical_time::MusicalTime::new(
                        &a.loaded.as_ref().unwrap().music,
                    )
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
    let music = &a.loaded.as_ref().unwrap().music;
    let keys = &a.loaded.as_ref().unwrap().keys;
    let catalog = Catalog::read(music).unwrap_or_default();
    let lanes = lanes(&c, id);
    let beacon = (p == Pane::Song)
        .then(|| super::super::beacon::step(a))
        .flatten();
    let mut first_cell = None;
    let mut clicked = None;
    let mut dropped = None;
    let mut moved = None;
    let mut commented = None;
    let mut scrolling = scrolling::Matrix::new(ui, p, position, end, cell_width, gap);
    let timeline = if p == Pane::Song {
        &mut a.patterns.workspace.timeline
    } else {
        &mut a.patterns.workspace.scene_timeline
    };
    if let Some(offset) = timeline.viewport.pending.take() {
        scrolling.offset = offset.x;
    }
    let mut repeat_action = None;
    let output = egui::ScrollArea::both()
        .drag_to_scroll(false)
        .id_salt((p.title(), "matrix"))
        .max_height(160.0)
        .horizontal_scroll_offset(scrolling.offset)
        .show_viewport(ui, |ui, viewport| {
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
            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                egui::Grid::new((p.title(), "cells"))
                    .min_col_width(0.0)
                    .spacing(egui::vec2(gap, ui.spacing().item_spacing.y))
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
                            for measure in first..end {
                                let covering =
                                    lane.and_then(|l| model::covering(&c, id, l, measure));
                                let continuation = covering
                                    .filter(|(_, start)| *start < measure)
                                    .map(|(_, start)| start);
                                let label = covering
                                    .filter(|_| continuation.is_none())
                                    .and_then(|(v, _)| {
                                        choices
                                            .iter()
                                            .find(|(p, _)| *p == v)
                                            .map(|(_, s)| s.as_str())
                                    })
                                    .unwrap_or(if continuation.is_some() { "—" } else { "·" });
                                let (rect, r) = ui.allocate_exact_size(
                                    egui::vec2(cell_width, 30.0),
                                    egui::Sense::click_and_drag(),
                                );
                                if let Some((_, start)) = covering {
                                    r.dnd_set_drag_payload(Placed(row, start));
                                }
                                let target = r.dnd_hover_payload::<PatternOrScene>().is_some()
                                    || r.dnd_hover_payload::<Placed>().is_some();
                                let faded =
                                    lane.is_some_and(|l| model::commented_out(&c, id, l, measure));
                                cell(
                                    ui,
                                    rect,
                                    label,
                                    selected == Some((row, measure)) || target,
                                    faded,
                                    |painter, area| {
                                        let Some((part, start)) = covering else {
                                            return;
                                        };
                                        let k = (measure - start) as usize;
                                        paint_part(
                                            painter,
                                            area,
                                            music,
                                            keys,
                                            &catalog,
                                            &c,
                                            part,
                                            Some((k, k + 1)),
                                        );
                                    },
                                );
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
                            ui.end_row();
                        }
                    });
            });
        });
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
    candidates::show(a, ui, p, &c, first_cell, beacon, (ime, was_focused));
}
