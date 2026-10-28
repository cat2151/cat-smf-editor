use super::*;
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
    if input_owner(a, ui.ctx()) || a.palette.is_open() {
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
    if s.help {
        ui.heading(p.title());
        egui::Grid::new((p.title(), "help")).show(ui, |ui| {
            for (key, label) in [
                ("← → / ↑ ↓", "小節 / レーン"),
                ("PageUp / PageDown", "部品選択"),
                ("Enter", "Put"),
                ("E", "本体編集"),
                ("Shift+E", "部品編集"),
                ("N", "新規pattern"),
                ("Delete", "配置解除"),
                ("Space", "再生"),
                ("Shift+Space", "停止"),
                ("P", "再生menu"),
                ("S", "Scene window"),
                ("Ctrl+Z", "Undo"),
                ("Ctrl+← → ↑ ↓", "配置移動"),
                ("Ctrl+/", "コメントアウト"),
                ("Esc", "終了"),
            ] {
                if key == "S" && p != Pane::Song {
                    continue;
                }
                ui.label(key);
                ui.label(label);
                ui.end_row();
            }
        });
        return;
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
    let first = s.measure.saturating_sub(3).max(1);
    let end = length.map_or(first + 12, |n| (n + 1).min(first + 12));
    let selected = (s.row, s.measure);
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
    egui::ScrollArea::both()
        .drag_to_scroll(false)
        .id_salt((p.title(), "matrix"))
        .max_height(160.0)
        .show(ui, |ui| {
            egui::Grid::new((p.title(), "cells")).show(ui, |ui| {
                ui.label("");
                for m in first..end {
                    ui.label(m.to_string());
                }
                ui.end_row();
                for row in 0..count {
                    let lane = lanes.get(row).copied();
                    ui.label((row + 1).to_string());
                    for measure in first..end {
                        let covering = lane.and_then(|l| model::covering(&c, id, l, measure));
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
                            egui::vec2(90.0, 30.0),
                            egui::Sense::click_and_drag(),
                        );
                        if let Some((_, start)) = covering {
                            r.dnd_set_drag_payload(Placed(row, start));
                        }
                        let target = r.dnd_hover_payload::<PatternOrScene>().is_some()
                            || r.dnd_hover_payload::<Placed>().is_some();
                        let faded = lane.is_some_and(|l| model::commented_out(&c, id, l, measure));
                        cell(
                            ui,
                            rect,
                            label,
                            selected == (row, measure) || target,
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
                        if (row, measure) == (0, 1) {
                            first_cell = Some(rect);
                        }
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new((p.title(), "cell", row, measure)), r.rect)
                        });
                        // Taking a payload clears it whatever its type, so ask first.
                        if r.dnd_hover_payload::<Placed>().is_some()
                            && let Some(from) = r.dnd_release_payload::<Placed>()
                        {
                            moved = Some((*from, row, measure));
                        } else if let Some(part) = r.dnd_release_payload::<PatternOrScene>() {
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
    if let Some((row, from, measure)) = clicked {
        let s = a.patterns.workspace.selection_mut(p);
        s.row = row;
        s.measure = measure;
        // Showing a body stops the transport, so playback starts after it.
        click_body(a, p);
        crate::play::arrangement::start_at(a, id, from);
    }
    if let Some((row, measure)) = commented {
        let s = a.patterns.workspace.selection_mut(p);
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
            s.row = row;
            s.measure = measure;
        }
        result(a, r);
    }
    if let Some((row, measure, part)) = dropped {
        let s = a.patterns.workspace.selection_mut(p);
        s.row = row;
        s.measure = measure;
        s.choice = Some(part);
        let r = place(a, p, Some(part));
        result(a, r);
    }
    ui.horizontal(|ui| {
        if let Some(Placed(row, start)) = trash(ui, p) {
            let r = mutate(a, |m| model::place(m, id, lanes[row], start, None));
            result(a, r);
        }
    });
    let s = a.patterns.workspace.selection_mut(p);
    let response = ui.add(egui::TextEdit::singleline(&mut s.query).id(query_id(p)));
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), "query")), response.rect));
    let confirm = !ime && response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    let cancel = !ime && was_focused && ui.input(|i| i.key_pressed(Key::Escape));
    if ime && was_focused && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) {
        response.request_focus();
    }
    s.search.update(&s.query);
    let filtered = choices
        .into_iter()
        .filter(|(_, label)| s.search.matches_text(label))
        .collect::<Vec<_>>();
    if !filtered.iter().any(|(part, _)| Some(*part) == s.choice) {
        s.choice = filtered.first().map(|(part, _)| *part);
    }
    let selected = s.choice;
    let mut clicked = None;
    let mut card = None;
    let mut edit = false;
    let focused = super::super::beacon::focused(ui.ctx());
    let music = &a.loaded.as_ref().unwrap().music;
    let keys = &a.loaded.as_ref().unwrap().keys;
    let catalog = Catalog::read(music).unwrap_or_default();
    egui::ScrollArea::vertical()
        .drag_to_scroll(false)
        .id_salt((p.title(), "parts"))
        .max_height(120.0)
        .show(ui, |ui| {
            for (part, label) in filtered {
                let (rect, r) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(240.0), 30.0),
                    egui::Sense::click_and_drag(),
                );
                cell(
                    ui,
                    rect,
                    &label,
                    Some(part) == selected,
                    false,
                    |painter, area| {
                        paint_part(painter, area, music, keys, &catalog, &c, part, None);
                    },
                );
                r.dnd_set_drag_payload(part);
                match beacon {
                    Some(super::super::beacon::Step::Edit(id))
                        if focused && part == PatternOrScene::Pattern(id) =>
                    {
                        let stroke = super::super::beacon::stroke(ui.ctx());
                        let b = ui.put(rect, egui::Button::new("Edit · Shift+E").stroke(stroke));
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new((p.title(), "beacon-edit")), b.rect)
                        });
                        edit |= b.clicked();
                    }
                    Some(super::super::beacon::Step::Place(id))
                        if part == PatternOrScene::Pattern(id) =>
                    {
                        card = Some(rect);
                    }
                    _ => {}
                }
                #[cfg(test)]
                if r.interact_rect.is_positive() {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            egui::Id::new((p.title(), "part", format!("{part:?}"))),
                            r.interact_rect,
                        )
                    });
                }
                if r.clicked() {
                    clicked = Some(part);
                }
            }
        });
    // Outside the scroll area, so it stays reachable however many parts there are.
    if button(ui, p, "new-pattern", "+")
        .on_hover_text("新規pattern (N)")
        .clicked()
    {
        library::add(a, ui.ctx());
        return;
    }
    if let (Some(card), Some(cell)) = (card, first_cell) {
        place_beacon(ui, card, cell);
    }
    if edit {
        edit_choice(a, ui.ctx(), p);
        return;
    }
    if let Some(part) = clicked {
        // Opening a pattern stops the transport, so the audition starts after it.
        let opened = match part {
            PatternOrScene::Pattern(id) => library::edit(a, id),
            PatternOrScene::Scene(id) => open_scene(a, id),
        };
        choose(a, p, part);
        if opened.is_err() {
            result(a, opened);
        } else if matches!(part, PatternOrScene::Pattern(_))
            && a.loaded
                .as_ref()
                .is_some_and(|l| l.patches[a.track].drums.is_some())
        {
            // As if `i` were pressed: the drum loop replaces the audition.
            crate::view::drums::open(a);
            presentation::focus_root(a, ui.ctx());
        }
    }
    if cancel {
        ui.ctx().memory_mut(|m| m.surrender_focus(query_id(p)));
    }
    if confirm {
        place_selected(a, p);
    }
}
/// The notes of `measures` (part-relative, half-open) of a pattern, or of a scene in the minimap colors.
#[allow(clippy::too_many_arguments)]
fn paint_part(
    painter: &egui::Painter,
    area: egui::Rect,
    music: &cat_smf_editor::model::Music,
    keys: &cat_smf_editor::score::key_signature::KeySignatures,
    catalog: &Catalog,
    c: &scene::Collection,
    part: PatternOrScene,
    measures: Option<(usize, usize)>,
) {
    use super::super::cards;
    match part {
        PatternOrScene::Pattern(id) => {
            if let Some(def) = catalog.patterns.get(&id) {
                let gray = |_, e: &cat_smf_editor::model::Event| {
                    crate::view::pitch_keyboard::note_color(e.data[0], None)
                };
                cards::paint_notes(painter, area, music, def, measures, &gray);
            }
        }
        PatternOrScene::Scene(id) => {
            if let Some(s) = c.scenes.iter().find(|s| s.id == id) {
                let color = cards::minimap_color(keys);
                cards::paint_scene(painter, area, music, catalog, s, measures, &color);
            }
        }
    }
}
/// The only pattern, its first cell and a drag arrow between them.
fn place_beacon(ui: &egui::Ui, card: egui::Rect, target: egui::Rect) {
    let stroke = super::super::beacon::stroke(ui.ctx());
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("song-beacon"),
    ));
    super::super::beacon::frame(&painter, card, stroke);
    super::super::beacon::frame(&painter, target, stroke);
    let from = card.center_top() - egui::vec2(0.0, 4.0);
    let to = target.center_bottom() + egui::vec2(0.0, 4.0);
    painter.arrow(from, to - from, stroke);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("song-beacon-place"), (card, target)));
}
/// A placement dropped here is removed.
fn trash(ui: &mut egui::Ui, p: Pane) -> Option<Placed> {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(40.0, 30.0), egui::Sense::hover());
    let hovered = r.dnd_hover_payload::<Placed>().is_some();
    cell(ui, rect, "", hovered, false, |painter, area| {
        painter.text(
            area.center(),
            egui::Align2::CENTER_CENTER,
            "🗑",
            egui::FontId::proportional(area.height()),
            ui.visuals().text_color(),
        );
    });
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), "trash")), r.rect));
    let _ = p;
    r.dnd_release_payload::<Placed>().map(|p| *p)
}
/// A part's name over its notes; selection is the frame and commenting out is faded
/// contents, not text.
fn cell(
    ui: &egui::Ui,
    rect: egui::Rect,
    label: &str,
    selected: bool,
    faded: bool,
    notes: impl FnOnce(&egui::Painter, egui::Rect),
) {
    let v = ui.visuals();
    let mut painter = ui.painter_at(rect);
    painter.rect(
        rect,
        3.0,
        v.faint_bg_color,
        if selected {
            egui::Stroke::new(2.0_f32, v.selection.stroke.color)
        } else {
            egui::Stroke::new(1.0_f32, v.widgets.noninteractive.bg_stroke.color)
        },
        egui::StrokeKind::Inside,
    );
    if faded {
        painter.multiply_opacity(0.3);
    }
    notes(&painter, rect.shrink(3.0));
    painter.text(
        rect.min + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::TextStyle::Small.resolve(ui.style()),
        v.text_color(),
    );
}
