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
        binding: &crate::view::timeline_controls::Binding<'_>,
        tempos: overview::Tempos<'_>,
        playing: Option<u64>,
    ) -> (egui::Rect, bool) {
        let end = binding.end as f32 * layout.scale;
        let (rect, remaining) = crate::view::timeline_minimap::area(area);
        let navigation = self
            .minimap_state
            .show_on(ui, rect, binding, playing, |painter, rect| {
                crate::view::piano_roll::minimap::draw(painter, rect, layout, end);
                overview::draw(painter, rect, end.max(1.0) / layout.scale, tempos);
            });
        self.minimap_rect = self.minimap_state.rect;
        if let Some(navigation) = &navigation {
            self.seek = navigation.seek.or(self.seek);
            self.pending_scroll = Some(navigation.scroll);
        }
        (remaining, navigation.is_some())
    }
}
