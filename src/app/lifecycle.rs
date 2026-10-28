//! App-wide busy -> interactive idle transition, after the ordinary UI work.
use super::App;
use crate::view;
use eframe::egui;

#[derive(Default)]
pub struct State {
    idle: bool,
    queued: Option<IdleNotification>,
}

struct IdleNotification {
    song: u64,
    frame: (egui::ViewportId, u64),
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    EnteredIdle,
}

impl State {
    pub fn interrupted(&mut self) {
        self.idle = false;
        self.queued = None;
    }

    fn observe(&mut self, idle: bool) -> Option<Event> {
        let entered = idle && !self.idle;
        self.idle = idle;
        if !idle {
            self.queued = None;
        }
        entered.then_some(Event::EnteredIdle)
    }
}

impl App {
    fn interactive_idle(&self, ctx: &egui::Context) -> bool {
        let idle = !self.startup.pending
            && !self.player_selection.setup.is_active()
            && !view::busy::pending(self)
            && !view::opening::pending(self)
            && !crate::play::transport::preparing(self)
            && !crate::play::sung::prerender::running(self)
            && !self.palette.is_open()
            && !self.file_open_requested
            && !self.config.navigation.close_approved
            && self.load_error.is_none()
            && view::commands::owner(self, ctx) == view::commands::Screen::Main;
        #[cfg(windows)]
        let idle = idle && !crate::player::resident_link::loading(self);
        idle
    }

    /// Deliver an app event only after the frame that became idle was presented.
    /// A layout retry is not a new frame; stale notifications never cross songs.
    pub(crate) fn frame_started(&mut self, ctx: &egui::Context) {
        let Some(event) = &self.lifecycle.queued else {
            return;
        };
        if event.frame == view::palette::frame_token(ctx) {
            return;
        }
        if event.song != self.song_id || !self.interactive_idle(ctx) {
            self.lifecycle.interrupted();
            return;
        }
        self.lifecycle.queued = None;
        self.on_idle(ctx);
    }

    /// This is the app's readiness boundary, not a dialog's opening heuristic.
    /// Called after the normal root panels and companion windows are drawn.
    pub(crate) fn frame_completed(&mut self, ctx: &egui::Context, event_list_drawn: bool) {
        let idle = event_list_drawn && self.interactive_idle(ctx);
        if self.lifecycle.observe(idle) == Some(Event::EnteredIdle) {
            self.lifecycle.queued = Some(IdleNotification {
                song: self.song_id,
                frame: view::palette::frame_token(ctx),
            });
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests;
