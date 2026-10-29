use super::*;

pub(super) fn show(
    a: &mut App,
    ui: &mut egui::Ui,
    p: Pane,
    c: &scene::Collection,
    first_cell: Option<egui::Rect>,
    beacon: Option<super::super::super::beacon::Step>,
    (ime, was_focused): (bool, bool),
) {
    let choices = choices(a, p);
    let s = a.patterns.workspace.selection_mut(p);
    let cells = s.cells(p);
    let response = ui.add(egui::TextEdit::singleline(&mut s.query).id(query_id(p)));
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new((p.title(), "query")), response.rect));
    let confirm = !ime && response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    let cancel = !ime && was_focused && ui.input(|i| i.key_pressed(Key::Escape));
    if ime && was_focused && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) {
        response.request_focus();
    }
    if response.gained_focus() {
        s.placing = false;
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
    let reveal = std::mem::take(&mut s.reveal_choice) || response.changed();
    let mut clicked = None;
    let mut card = None;
    let mut edit = false;
    let focused = super::super::super::beacon::focused(ui.ctx());
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
                    Some(part) == selected && (p == Pane::Scene || !cells),
                    false,
                    |painter, area| {
                        paint_part(painter, area, music, keys, &catalog, c, part, None);
                    },
                );
                if cells && Some(part) == selected && p == Pane::Song {
                    ui.painter().circle_filled(
                        rect.right_center() - egui::vec2(7.0, 0.0),
                        3.0,
                        ui.visuals().selection.stroke.color,
                    );
                }
                if Some(part) == selected && !cells && reveal {
                    r.scroll_to_me(Some(egui::Align::Center));
                }
                r.dnd_set_drag_payload(part);
                match beacon {
                    Some(super::super::super::beacon::Step::Edit(id))
                        if focused && !cells && part == PatternOrScene::Pattern(id) =>
                    {
                        let stroke = super::super::super::beacon::stroke(ui.ctx());
                        let b = ui.put(rect, egui::Button::new("Edit · E").stroke(stroke));
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new((p.title(), "beacon-edit")), b.rect)
                        });
                        edit |= b.clicked();
                    }
                    Some(super::super::super::beacon::Step::Place(id))
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
        place_beacon(ui, card, cell, cells);
    }
    if edit {
        edit_choice(a, ui.ctx(), p);
        return;
    }
    if let Some(part) = clicked {
        let s = a.patterns.workspace.selection_mut(p);
        s.placing = false;
        s.choice = Some(part);
        let opened = match part {
            PatternOrScene::Pattern(id) => {
                let opened = library::edit(a, id).and_then(|()| preview::start(a, id));
                if opened.is_ok() {
                    presentation::focus_root(a, ui.ctx());
                }
                opened
            }
            PatternOrScene::Scene(id) => {
                let opened = open_scene(a, id);
                if opened.is_ok() {
                    crate::play::arrangement::start(a, Some(id));
                }
                opened
            }
        };
        result(a, opened);
    }
    if cancel {
        ui.ctx().memory_mut(|m| m.surrender_focus(query_id(p)));
    }
    if confirm {
        super::super::commands::confirm(a, p);
    }
}
