//! Scalar time state, independent of the curve's vertical coordinate system.
use super::*;

pub(crate) struct Horizontal {
    pub zoom: f32,
    pub scroll: f32,
    pub width: f32,
    pub pending_scroll: Option<f32>,
    pub seek: Option<u64>,
    pub screen: Option<Screen>,
    pub horizontal_zoom: Zoom,
    pub minimap_state: Minimap,
    pub minimap_rect: Option<egui::Rect>,
    pub horizontal_bar: Option<egui::Rect>,
    viewport: Option<egui::Rect>,
}
impl Default for Horizontal {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            scroll: 0.0,
            width: 0.0,
            pending_scroll: None,
            seek: None,
            screen: None,
            horizontal_zoom: Default::default(),
            minimap_state: Default::default(),
            minimap_rect: None,
            horizontal_bar: None,
            viewport: None,
        }
    }
}
impl Horizontal {
    pub fn suspend(&mut self) {
        self.minimap_state = Default::default();
        self.horizontal_zoom = Default::default();
        self.seek = None;
        self.pending_scroll = None;
    }
    pub fn begin(&mut self, screen: Screen) {
        assert!(Screen::ALL.contains(&screen));
        if self.screen != Some(screen) {
            self.suspend();
        }
        self.screen = Some(screen);
    }
    pub fn binding<'a>(&self, axis: &'a dyn timeline_repeat::TickAxis, end: u64) -> Binding<'a> {
        Binding {
            screen: self
                .screen
                .expect("register timeline before drawing controls"),
            axis,
            viewport: self.viewport.unwrap_or(egui::Rect::NOTHING),
            scroll: self.scroll,
            end,
            zoom_range: self.horizontal_zoom.range(),
        }
    }
    pub fn finish(&mut self, rect: egui::Rect, offset: f32) {
        let (rect, offset) = super::finish(rect, offset);
        self.scroll = offset;
        self.width = rect.width().max(0.0);
        self.viewport = Some(rect);
    }
    pub fn zoom_time(&mut self, to: f32, anchor: f32) {
        let (next, offset) = super::zoom(self.scroll, self.width, self.zoom, to, anchor);
        self.zoom = next;
        self.pending_scroll = Some(offset);
    }
    pub fn time_zoom(&self) -> f32 {
        self.zoom
    }
}
