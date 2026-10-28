use super::*;
use cat_smf_editor::edit;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Repeat,
    DeleteEvents,
    Init,
    DeleteTrack,
}

fn sources(app: &App) -> Vec<usize> {
    let Some(range) = range(app) else {
        return vec![];
    };
    let l = app.loaded.as_ref().unwrap();
    if !app.note_mode {
        return range.collect();
    }
    // Iterate source events rather than potentially billions of empty beat rows.
    l.display_rows[app.track].events_in_rows(range).collect()
}

pub(super) fn ticks(app: &App) -> Option<(u64, u64)> {
    let range = range(app)?;
    let l = app.loaded.as_ref()?;
    let events = &l.visible().tracks[app.track].events;
    let tick = |r| crate::cursor::split::row_tick_loaded(l, app.note_mode, app.track, r);
    let a = tick(range.start);
    let mut b = tick(range.end - 1);
    for i in sources(app) {
        b = b.max(
            events[i]
                .tick
                .saturating_add(l.note_views[app.track].durations[i].unwrap_or(0)),
        );
    }
    if range.end < count(app) {
        b = b.max(tick(range.end));
    }
    if a == b {
        b = l
            .time
            .map(app.track)
            .and_then(|m| {
                let p = m.position(b);
                m.adjacent_beat(&p, true, u64::MAX)
                    .and_then(|p| m.tick_at(&p))
            })
            .unwrap_or(b.saturating_add(1));
    }
    Some((a, b))
}

pub(super) fn apply(app: &mut App, action: Action) {
    if matches!(action, Action::Repeat) {
        if let Some(range) = ticks(app) {
            crate::play::transport::repeat::start(app, range);
        }
        app.event_selection = State::default();
        return;
    }
    let start = ticks(app).map_or(0, |(a, _)| a);
    let track = app.track;
    let selected = sources(app);
    let notes = app.note_mode;
    if matches!(action, Action::DeleteEvents) {
        let result = app.edit_music(|music| {
            edit::event_range::delete(&mut music.tracks[track], &selected, notes)
        });
        match result {
            Ok(()) => {
                app.event_selection = State::default();
                crate::cursor::moves::at_tick(app, start);
            }
            Err(error) => app.load_error = Some(error),
        }
        return;
    }
    let all = whole(app);
    if matches!(action, Action::Init | Action::DeleteTrack) && !all {
        return;
    }
    let result = app.edit_tracks(|music| match action {
        Action::DeleteEvents => unreachable!(),
        Action::Init => edit::event_range::initialize(music, track),
        Action::DeleteTrack => {
            if music.tracks.len() == 1 {
                return Err("Cannot delete the last SMF track".into());
            }
            music.tracks.remove(track);
            Ok(())
        }
        Action::Repeat => unreachable!(),
    });
    match result {
        Ok(()) => {
            app.event_selection = State::default();
            if matches!(action, Action::DeleteTrack) {
                app.track_view.split = None;
            }
            let target = track.min(app.loaded.as_ref().unwrap().music.tracks.len() - 1);
            app.select_track(target);
            let last = app.loaded.as_ref().unwrap().music.tracks[target]
                .events
                .last()
                .map_or(0, |e| e.tick);
            crate::cursor::moves::at_tick(
                app,
                if matches!(action, Action::DeleteEvents) {
                    start.min(last)
                } else {
                    0
                },
            );
        }
        Err(error) => app.load_error = Some(error),
    }
}
