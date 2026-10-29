//! Read-only arrangement cards and drag/drop affordances.
use super::*;
/// The notes of `measures` (part-relative, half-open) of a pattern, or of a scene in the minimap colors.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_part(
    painter: &egui::Painter,
    area: egui::Rect,
    music: &cat_smf_editor::model::Music,
    keys: &cat_smf_editor::score::key_signature::KeySignatures,
    catalog: &Catalog,
    c: &scene::Collection,
    part: PatternOrScene,
    measures: Option<(usize, usize)>,
) {
    use crate::view::patterns::cards;
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
pub(super) fn place_beacon(ui: &egui::Ui, card: egui::Rect, target: egui::Rect, cells: bool) {
    let stroke = crate::view::patterns::beacon::stroke(ui.ctx());
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("song-beacon"),
    ));
    crate::view::patterns::beacon::frame(&painter, if cells { target } else { card }, stroke);
    let from = card.center_top() - egui::vec2(0.0, 4.0);
    let to = target.center_bottom() + egui::vec2(0.0, 4.0);
    painter.arrow(from, to - from, stroke);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("song-beacon-place"), (card, target)));
}
pub(super) enum Discard {
    Placement(Placed),
    Pattern(u64),
}
/// Placed blocks remove an occurrence; Song candidates delete the pattern itself.
pub(super) fn trash(ui: &mut egui::Ui, p: Pane) -> Option<Discard> {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(40.0, 30.0), egui::Sense::hover());
    let pattern = p == Pane::Song
        && r.dnd_hover_payload::<PatternOrScene>()
            .is_some_and(|part| matches!(*part, PatternOrScene::Pattern(_)));
    let hovered = r.dnd_hover_payload::<Placed>().is_some() || pattern;
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
    let r = r.on_hover_text(if p == Pane::Song {
        "配置解除 / 候補pattern削除 (Shift+Delete)"
    } else {
        "配置解除 (Delete)"
    });
    // A mismatched release consumes the payload too; inspect its type first.
    if pattern {
        r.dnd_release_payload::<PatternOrScene>()
            .and_then(|part| match *part {
                PatternOrScene::Pattern(id) => Some(Discard::Pattern(id)),
                PatternOrScene::Scene(_) => None,
            })
    } else if r.dnd_hover_payload::<Placed>().is_some() {
        r.dnd_release_payload::<Placed>()
            .map(|p| Discard::Placement(*p))
    } else {
        None
    }
}
/// A part's name over its notes; selection is the frame and commenting out is faded
/// contents, not text.
pub(super) fn cell(
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
    if rect.width() > 6.0 && rect.height() > 6.0 {
        notes(&painter, rect.shrink(3.0));
    }
    painter.text(
        rect.min + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::TextStyle::Small.resolve(ui.style()),
        v.text_color(),
    );
}
