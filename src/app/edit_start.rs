//! Policy evaluated on app idle: offer an entry only for an untouched new song.
use super::App;
use eframe::egui;

#[derive(Default)]
pub struct State {
    new_document: Option<u64>,
}

impl State {
    pub(super) fn new_document(&mut self, song: u64) {
        self.new_document = Some(song);
    }
}

impl App {
    pub(super) fn on_idle(&mut self, ctx: &egui::Context) {
        let Some(song) = self.edit_start.new_document.take() else {
            return;
        };
        if song == self.song_id && self.saved.is_untitled() && !self.undo.available() {
            crate::view::edit_style::open(self);
            ctx.request_repaint();
        }
    }
}
