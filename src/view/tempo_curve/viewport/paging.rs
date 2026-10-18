//! Half a viewport per click along the scrollbar axis.
use super::*;

impl Viewport {
    pub(crate) fn horizontal_pages(&mut self, ui: &mut egui::Ui, bar: egui::Rect) {
        self.horizontal_bar = Some(bar);
        if let Some(direction) = timeline_zoom::page_buttons(ui, bar, false, "tempo-page") {
            self.pending_scroll =
                Some((self.scroll + direction * self.width * timeline_zoom::PAGE).max(0.0));
        }
    }
}
