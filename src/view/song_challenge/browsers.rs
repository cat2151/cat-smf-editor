use super::{App, State};
use crate::view::patterns::workspace::{self, Pane};
use eframe::egui;
use std::collections::VecDeque;

#[cfg(windows)]
use cat_smf_editor::resident_ipc::Open;

pub(crate) struct Queue {
    remaining: VecDeque<usize>,
    #[cfg(windows)]
    awaiting: Option<Open>,
}

impl Queue {
    pub(super) fn new(containers: [usize; 4]) -> Self {
        Self {
            remaining: containers.into(),
            #[cfg(windows)]
            awaiting: None,
        }
    }
}

pub(super) fn open_next(app: &mut App, ctx: &egui::Context) {
    let State::Running { browsers, .. } = &mut app.song_challenge else {
        return;
    };
    let Some(track) = browsers.remaining.pop_front() else {
        workspace::focus(app, ctx, Pane::Song);
        return;
    };
    // Bind the resident target without opening a pattern editor or moving Song's focus.
    app.set_track(track);
    crate::player::bulk::request(app);
    #[cfg(windows)]
    if let State::Running { browsers, .. } = &mut app.song_challenge {
        browsers.awaiting = if app.player_selection.error.is_none() {
            crate::player::resident_link::target(&app.player_selection)
        } else {
            None
        };
    }
}

#[cfg(windows)]
pub(crate) fn awaiting(app: &App, target: Open) -> bool {
    matches!(&app.song_challenge, State::Running { browsers, .. }
        if browsers.awaiting == Some(target))
}

#[cfg(windows)]
pub(crate) fn closed(app: &mut App, ctx: &egui::Context, open: Open) -> bool {
    let song = match open {
        Open::Kit { song, .. } | Open::Layer { song, .. } => song,
        _ => return false,
    };
    if song != app.song_id {
        return false;
    }
    let State::Running { browsers, .. } = &mut app.song_challenge else {
        return false;
    };
    if browsers.awaiting != Some(open) {
        return false;
    }
    browsers.awaiting = None;
    open_next(app, ctx);
    true
}
