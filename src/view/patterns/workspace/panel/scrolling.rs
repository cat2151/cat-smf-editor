//! Keep measure one at the content origin, independently of the cell cursor.
use super::*;

#[derive(Clone, Copy, Default)]
struct State {
    position: Option<(usize, u64)>,
    offset: f32,
    width: f32,
    end: u64,
    head: Option<f32>,
}

pub(super) struct Matrix {
    state: State,
    pub offset: f32,
    stride: f32,
    label: f32,
    end: u64,
}

impl Matrix {
    pub fn new(
        ui: &egui::Ui,
        pane: Pane,
        position: (usize, u64),
        end: u64,
        cell_width: f32,
    ) -> Self {
        let id = ui.id().with((pane.title(), "matrix-position"));
        let mut state = ui
            .ctx()
            .data(|d| d.get_temp::<State>(id))
            .unwrap_or_default();
        // Measures abut so a pattern's notes run on across bar lines; only the row label keeps a gap.
        let stride = cell_width;
        let label = 24.0 + ui.spacing().item_spacing.x;
        let mut offset = state.offset;
        // Reveal a changed selection once. Manual scrolling must remain independent
        // of the selected cell, including when returning to measure one.
        if state.position != Some(position) {
            let left = label + position.1.saturating_sub(1) as f32 * stride;
            let right = left + cell_width;
            let width = if state.width > 0.0 {
                state.width
            } else {
                ui.available_width()
            };
            if left < offset {
                offset = left;
            } else if right > offset + width {
                offset = (right - width).max(0.0);
            }
        }
        state.position = Some(position);
        // Keep the reachable range when the cursor moves back to earlier cells.
        let end = if pane == Pane::Song {
            end.max(state.end)
        } else {
            end
        };
        Self {
            state,
            offset,
            stride,
            label,
            end,
        }
    }

    /// Brings back a playhead that just left the view, at the left edge. A view scrolled away
    /// from it stays, and a held pointer keeps the content under it until release.
    pub fn turn_page(&mut self, ui: &egui::Ui, head: Option<f32>, unmoved: bool) {
        if ui.input(|i| i.pointer.any_down()) {
            return;
        }
        let view = self.offset..=self.offset + self.state.width;
        if let (Some(before), Some(now)) = (self.state.head, head)
            && unmoved
            && view.contains(&before)
            && !view.contains(&now)
        {
            self.offset = (now - self.label).max(0.0);
        }
        self.state.head = head;
    }

    pub fn visible(&mut self, ui: &mut egui::Ui, viewport: egui::Rect) -> (egui::Rect, u64, u64) {
        let width = self.label + self.end.saturating_sub(1) as f32 * self.stride;
        ui.set_min_width(width);
        let first = (((viewport.left() - self.label).max(0.0) / self.stride).floor() as u64)
            .saturating_add(1);
        let end = ((viewport.right() / self.stride).ceil() as u64)
            .saturating_add(2)
            .min(self.end);
        let mut area = ui.available_rect_before_wrap();
        area.min.x += first.saturating_sub(1) as f32 * self.stride;
        (area, first, end)
    }

    pub fn repeat(
        &self,
        ui: &mut egui::Ui,
        music: &cat_smf_editor::model::Music,
        state: &mut super::super::timeline::State,
        range: Option<(u64, u64)>,
    ) -> Option<crate::view::timeline_repeat::Action> {
        let time = cat_smf_editor::score::musical_time::MusicalTime::new(music);
        let map = time.map(0)?;
        let axis = super::super::timeline::Cells {
            map,
            stride: self.stride,
            label: 0.0,
        };
        let (whole, _) = ui.allocate_exact_size(
            egui::vec2(
                self.label + self.end.saturating_sub(1) as f32 * self.stride,
                crate::view::timeline_repeat::HEIGHT,
            ),
            egui::Sense::hover(),
        );
        let rect = whole.with_min_x(whole.left() + self.label);
        let action = state
            .viewport
            .binding(&axis, 0)
            .repeat(ui, rect, range, &mut state.repeat);
        if let Some(tick) = state.anchor {
            let x = rect.left() + crate::view::timeline_repeat::TickAxis::x(&axis, tick);
            ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(2.0_f32, crate::view::timeline_repeat::RANGE),
            );
        }
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new((Pane::Song.title(), "repeat")), rect));
        action
    }
    pub fn finish_timeline<R>(
        &self,
        output: &egui::scroll_area::ScrollAreaOutput<R>,
        state: &mut super::super::timeline::State,
    ) {
        state.cells = Some((self.stride, self.label));
        state.viewport.scroll = output.state.offset;
        state.viewport.panel = Some(output.inner_rect);
    }
    pub fn label(&self) -> f32 {
        self.label
    }
    pub fn row_label(&self, ui: &mut egui::Ui, text: &str) {
        ui.add_sized(
            [self.label, ui.text_style_height(&egui::TextStyle::Body)],
            egui::Label::new(text),
        );
    }

    pub fn finish<R>(
        &mut self,
        ui: &egui::Ui,
        pane: Pane,
        output: &egui::scroll_area::ScrollAreaOutput<R>,
    ) {
        self.state.offset = output.state.offset.x;
        self.state.width = output.inner_rect.width();
        self.state.end = self.end;
        ui.ctx().data_mut(|d| {
            d.insert_temp(ui.id().with((pane.title(), "matrix-position")), self.state)
        });
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            d.insert_temp(
                egui::Id::new((pane.title(), "matrix-viewport")),
                output.inner_rect,
            );
            d.insert_temp(egui::Id::new((pane.title(), "matrix-scroll")), output.id);
        });
    }
}
