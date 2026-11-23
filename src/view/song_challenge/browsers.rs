use super::{App, State};
use crate::play::{playback::Mode, transport};
use crate::view::patterns::workspace::{self, Pane};
use cat_smf_editor::score::scene::arrangement;
use cmrt_patches::PatchRole;
use eframe::egui;
use std::collections::VecDeque;

#[cfg(windows)]
use cat_smf_editor::resident_ipc::Open;

pub(crate) struct Queue {
    first: First,
    remaining: VecDeque<((usize, u64), Option<PatchRole>)>,
    preview: Option<u64>,
    #[cfg(windows)]
    awaiting: Option<Open>,
}

enum First {
    Restoring,
    Ready,
    Opened,
}

impl Queue {
    pub(super) fn new(containers: [(usize, u64); 4]) -> Self {
        Self {
            first: First::Restoring,
            remaining: containers
                .into_iter()
                .zip([
                    None,
                    Some(PatchRole::Bass),
                    Some(PatchRole::Chord),
                    Some(PatchRole::Lead),
                ])
                .collect(),
            preview: None,
            #[cfg(windows)]
            awaiting: None,
        }
    }
}

pub(in crate::view) fn presentation_restored(app: &mut App, ctx: &egui::Context) {
    if let State::Running { browsers, .. } = &mut app.song_challenge
        && matches!(browsers.first, First::Restoring)
    {
        browsers.first = First::Ready;
        ctx.request_repaint();
    }
}

pub(super) fn open_first(app: &mut App, ctx: &egui::Context) {
    // A second egui pass in the same frame still precedes native viewport updates.
    if ctx.output(|output| output.num_completed_passes == 0)
        && let State::Running { browsers, .. } = &mut app.song_challenge
        && matches!(browsers.first, First::Ready)
    {
        browsers.first = First::Opened;
        open_next(app, ctx);
    }
}

fn open_next(app: &mut App, ctx: &egui::Context) {
    let State::Running { browsers, .. } = &mut app.song_challenge else {
        return;
    };
    let Some(((track, pattern), role)) = browsers.remaining.pop_front() else {
        end_preview(app);
        let music = &app.loaded.as_ref().unwrap().music;
        match arrangement::occurrences(music, None) {
            Ok((_, end)) => transport::repeat::start_song(app, (0, end)),
            Err(error) => app.load_error = Some(error),
        }
        workspace::focus(app, ctx, Pane::Song);
        return;
    };
    // Bind the resident target without opening a pattern editor or moving Song's focus.
    app.set_track(track);
    if let Err(error) = transport::repeat::start_pattern(app, pattern) {
        end_preview(app);
        app.load_error = Some(error);
        return;
    }
    if let State::Running { browsers, .. } = &mut app.song_challenge {
        browsers.preview = Some(pattern);
    }
    if let Some(role) = role {
        crate::player::selection::open_random_role(app, role);
    } else {
        crate::player::bulk::request(app);
    }
    if app.player_selection.error.is_some() {
        end_preview(app);
        return;
    }
    #[cfg(windows)]
    if let State::Running { browsers, .. } = &mut app.song_challenge {
        browsers.awaiting = crate::player::resident_link::target(&app.player_selection);
    }
}

pub(super) fn end_preview(app: &mut App) {
    if let State::Running { browsers, .. } = &mut app.song_challenge
        && let Some(pattern) = browsers.preview.take()
        && app.playback.repeat.is_some()
        && app.playback.repeat_mode == Some(Mode::Pattern(pattern))
    {
        transport::stop(app);
        app.playback.repeat_mode = None;
    }
}

#[cfg(windows)]
pub(crate) fn disconnected(app: &mut App, open: Open) {
    if awaiting(app, open) {
        end_preview(app);
        if let State::Running { browsers, .. } = &mut app.song_challenge {
            browsers.awaiting = None;
        }
    }
}

#[cfg(windows)]
pub(crate) fn awaiting(app: &App, target: Open) -> bool {
    matches!(&app.song_challenge, State::Running { browsers, .. }
        if browsers.awaiting == Some(target))
}

#[cfg(windows)]
pub(crate) fn closed(app: &mut App, ctx: &egui::Context, open: Open, confirmed: bool) -> bool {
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
    if confirmed {
        open_next(app, ctx);
    } else {
        end_preview(app);
        workspace::focus(app, ctx, Pane::Song);
    }
    true
}
