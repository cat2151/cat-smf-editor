//! Fixed song overview and navigation shared by the two timeline overlays.
use super::piano_roll::Layout;
use eframe::egui::{self, Color32};

const COLOR: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);
const INSET: f32 = 8.0;

#[derive(Default)]
pub(crate) struct State {
    pub rect: Option<egui::Rect>,
    drag: Option<f32>,
}

pub(crate) struct View {
    pub end: f32,
    pub zoom: f32,
    pub scroll: f32,
    pub width: f32,
    /// Playback position in the unzoomed layout coordinates.
    pub playing: Option<f32>,
}

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Center(f32),
    Scroll(f32),
}

impl State {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        layout: &Layout,
        view: View,
        overlay: impl FnOnce(&egui::Painter, egui::Rect),
    ) -> (egui::Rect, Option<Action>) {
        let height = (area.height() * 0.18).clamp(24.0, 56.0);
        let rect = egui::Rect::from_min_max(
            area.min + egui::vec2(INSET, 2.0),
            egui::pos2(
                (area.right() - 16.0 - INSET).max(area.left() + INSET + 1.0),
                area.top() + height,
            ),
        );
        self.rect = Some(rect);
        let end = view.end.max(1.0);
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
        painter.rect_filled(rect, 2.0, Color32::from_gray(28));
        for (i, (_, note)) in layout.notes.iter().enumerate() {
            let point = |p: egui::Pos2| {
                rect.min
                    + egui::vec2(
                        p.x / end * rect.width(),
                        p.y / layout.size.y * rect.height(),
                    )
            };
            let mut note = egui::Rect::from_min_max(point(note.min), point(note.max));
            note.max.x = note.max.x.max(note.min.x + 1.0);
            note.max.y = note.max.y.max(note.min.y + 1.0);
            painter.rect_filled(note, 0.0, layout.colors[i].gamma_multiply(0.65));
        }
        overlay(&painter, rect);
        let x = |offset: f32| {
            (rect.left() + offset / view.zoom / end * rect.width()).clamp(rect.left(), rect.right())
        };
        let visible = egui::Rect::from_x_y_ranges(
            x(view.scroll)..=x(view.scroll + view.width),
            rect.y_range(),
        );
        painter.rect_filled(visible, 0.0, COLOR.gamma_multiply(0.12));
        painter.rect_stroke(
            visible,
            0.0,
            egui::Stroke::new(1.0_f32, COLOR),
            egui::StrokeKind::Inside,
        );
        let response = ui.interact(
            rect,
            ui.id().with("timeline-minimap"),
            egui::Sense::click_and_drag(),
        );
        let local = |p: egui::Pos2| ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * end;
        let span = view.width / view.zoom;
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.drag = ui.input(|i| i.pointer.press_origin()).map(|pos| {
                let start = local(pos);
                let left = view.scroll / view.zoom;
                if (left..=left + span).contains(&start) {
                    start - left
                } else {
                    span / 2.0
                }
            });
        }
        let mut action = None;
        if let Some((grab, pos)) = self.drag.zip(response.interact_pointer_pos()) {
            let scroll = (local(pos) - grab).clamp(0.0, (end - span).max(0.0));
            action = Some(Action::Scroll(scroll));
        }
        if let Some(position) = view.playing {
            painter.vline(
                (rect.left() + position / end * rect.width()).clamp(rect.left(), rect.right()),
                rect.y_range(),
                egui::Stroke::new(2.0_f32, super::timeline_repeat::PLAYHEAD),
            );
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            action = Some(Action::Center(local(pos)));
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.drag = None;
        }
        (
            egui::Rect::from_min_max(egui::pos2(area.left(), rect.bottom() + 4.0), area.max),
            action,
        )
    }
}

pub(crate) mod tempos;
