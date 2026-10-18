//! Whether the timeline overlays scroll with the playback position this frame.
use eframe::egui;

/// Playback tick to keep centered this frame, or `None` to leave the scroll alone.
pub(crate) struct Follow(Option<u64>);

impl Follow {
    /// Follows only while follow is on and no pointer button is held or just released,
    /// so content being dragged stays under the pointer.
    pub fn new(ctx: &egui::Context, follow: bool, playing: Option<u64>) -> Self {
        Self(
            playing.filter(|_| {
                follow && !ctx.input(|i| i.pointer.any_down() || i.pointer.any_released())
            }),
        )
    }

    /// After the minimap moved the view, turns follow off so the view stays where it was moved.
    pub fn minimap(&mut self, follow: &mut bool, navigated: bool) {
        if navigated {
            *follow = false;
            self.0 = None;
        }
    }

    pub fn tick(&self) -> Option<u64> {
        self.0
    }
}
