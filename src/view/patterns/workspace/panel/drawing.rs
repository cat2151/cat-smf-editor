//! Read-only arrangement cards and drag/drop affordances.
use super::*;
/// The notes of `measures` (part-relative, half-open) of a pattern in its container's color,
/// or of a scene in the minimap colors.
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
                let color = crate::view::track_color(def.track);
                cards::paint_container_notes(painter, area, music, def, measures, color);
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
const PLACED_FILL_OPACITY: f32 = 0.18;
/// Background of a placed part: its container's color, or the plain cell for a scene.
pub(super) fn placed_fill(ui: &egui::Ui, catalog: &Catalog, part: PatternOrScene) -> egui::Color32 {
    match part {
        PatternOrScene::Pattern(id) => catalog.patterns.get(&id).map(|p| p.track),
        PatternOrScene::Scene(_) => None,
    }
    .map_or(ui.visuals().faint_bg_color, |track| {
        crate::view::track_color(track).gamma_multiply(PLACED_FILL_OPACITY)
    })
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
    cell(ui, rect, "", hovered, |painter, area| {
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
/// One placement across its shown measures: a single frame over its bar lines, so a long
/// pattern reads apart from repeats of a short one. The cursor anywhere inside selects
/// the frame.
pub(super) struct Block<'l> {
    pub start: u64,
    rect: egui::Rect,
    bars: Vec<f32>,
    background: egui::layers::ShapeIdx,
    fill: egui::Color32,
    selected: bool,
    faded: bool,
    label: &'l str,
}
impl<'l> Block<'l> {
    /// Reserves the background below the measures painted next.
    pub fn begin(
        ui: &egui::Ui,
        start: u64,
        rect: egui::Rect,
        label: &'l str,
        fill: egui::Color32,
        faded: bool,
    ) -> Self {
        Self {
            start,
            rect,
            bars: Vec::new(),
            background: ui.painter().add(egui::Shape::Noop),
            fill,
            selected: false,
            faded,
            label,
        }
    }
    pub fn extend(&mut self, rect: egui::Rect, cursor: bool) {
        if rect.center().x > self.rect.right() {
            self.bars.push((self.rect.right() + rect.left()) / 2.0);
        }
        self.rect = self.rect.union(rect);
        self.selected |= cursor;
    }
    pub fn finish(self, ui: &egui::Ui) {
        let v = ui.visuals();
        let painter = ui
            .painter()
            .with_clip_rect(ui.clip_rect().intersect(self.rect));
        painter.set(
            self.background,
            egui::Shape::rect_filled(
                self.rect,
                3.0,
                if self.faded {
                    self.fill.gamma_multiply(0.3)
                } else {
                    self.fill
                },
            ),
        );
        let bar = v.widgets.noninteractive.bg_stroke.color.gamma_multiply(0.5);
        for x in self.bars {
            painter.vline(
                x,
                self.rect.y_range().shrink(4.0),
                egui::Stroke::new(1.0_f32, bar),
            );
        }
        painter.rect_stroke(
            self.rect,
            3.0,
            frame(v, self.selected),
            egui::StrokeKind::Inside,
        );
        let opacity = if self.faded { 0.3 } else { 1.0 };
        name_tag(ui, &painter, self.rect, self.label, opacity);
    }
}
fn frame(v: &egui::Visuals, selected: bool) -> egui::Stroke {
    if selected {
        egui::Stroke::new(2.0_f32, v.selection.stroke.color)
    } else {
        egui::Stroke::new(1.0_f32, v.widgets.noninteractive.bg_stroke.color)
    }
}
/// The notes span the part's height, so its name needs its own ground.
fn name_tag(ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect, label: &str, opacity: f32) {
    let v = ui.visuals();
    let text = v.strong_text_color().gamma_multiply(opacity);
    let font = egui::TextStyle::Small.resolve(ui.style());
    let galley = painter.layout_no_wrap(label.to_owned(), font, text);
    let at = rect.min + egui::vec2(4.0, 2.0);
    painter.rect_filled(
        egui::Rect::from_min_size(at, galley.size()).expand(1.0),
        2.0,
        v.panel_fill.gamma_multiply(0.7 * opacity),
    );
    painter.galley(at, galley, text);
}
/// No side inset: an inset would read as a rest at every bar line.
fn notes_area(rect: egui::Rect) -> egui::Rect {
    rect.shrink2(egui::vec2(0.0, rect.height() * 0.05))
}
/// One measure inside a [`Block`]: the cursor's measure is tinted, a drop target framed.
pub(super) fn measure_in_block(
    ui: &egui::Ui,
    rect: egui::Rect,
    cursor: bool,
    target: bool,
    faded: bool,
    notes: impl FnOnce(&egui::Painter, egui::Rect),
) {
    let v = ui.visuals();
    let mut painter = ui.painter_at(rect);
    if cursor {
        painter.rect_filled(rect, 3.0, v.selection.bg_fill.gamma_multiply(0.35));
    }
    if target {
        painter.rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(2.0_f32, v.selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    if faded {
        painter.multiply_opacity(0.3);
    }
    if rect.width() > 6.0 && rect.height() > 6.0 {
        notes(&painter, notes_area(rect));
    }
}
/// A candidate, drawn like its placement in the matrix.
pub(super) fn part_card(
    ui: &egui::Ui,
    rect: egui::Rect,
    label: &str,
    fill: egui::Color32,
    selected: bool,
    notes: impl FnOnce(&egui::Painter, egui::Rect),
) {
    let painter = ui.painter_at(rect);
    painter.rect(
        rect,
        3.0,
        fill,
        frame(ui.visuals(), selected),
        egui::StrokeKind::Inside,
    );
    if rect.width() > 6.0 && rect.height() > 6.0 {
        notes(&painter, notes_area(rect));
    }
    name_tag(ui, &painter, rect, label, 1.0);
}
/// A plain cell: an empty matrix position or the trash.
pub(super) fn cell(
    ui: &egui::Ui,
    rect: egui::Rect,
    label: &str,
    selected: bool,
    notes: impl FnOnce(&egui::Painter, egui::Rect),
) {
    let v = ui.visuals();
    let painter = ui.painter_at(rect);
    painter.rect(
        rect,
        3.0,
        v.faint_bg_color,
        frame(v, selected),
        egui::StrokeKind::Inside,
    );
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
