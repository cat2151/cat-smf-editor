//! Song-wide note overview, independent of timeline zoom and scroll.
use super::*;

pub(super) use crate::view::timeline_minimap::tempos as overview;

impl Viewport {
    /// Returns the remaining content area and whether navigation was requested.
    pub(crate) fn minimap(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        layout: &Layout,
        end: f32,
        tempos: overview::Tempos<'_>,
        playing: Option<u64>,
    ) -> (egui::Rect, bool) {
        use crate::view::timeline_minimap::{View, Zoomed};
        let (remaining, action) = self.minimap_state.show(
            ui,
            area,
            layout,
            View {
                end,
                playing: playing.map(|tick| tick as f32 * layout.scale),
                zoom: self.zoom,
                scroll: self.scroll,
                width: self.width,
            },
            |painter, rect| overview::draw(painter, rect, end.max(1.0) / layout.scale, tempos),
        );
        self.minimap_rect = self.minimap_state.rect;
        if let Some(action) = action {
            let navigation = action.navigate(layout.scale, self.width, &Zoomed(self.zoom));
            self.seek = navigation.seek.or(self.seek);
            self.pending_scroll = Some(navigation.scroll);
        }
        (remaining, action.is_some())
    }
}
