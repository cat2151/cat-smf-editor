use super::*;
use cat_smf_editor::edit;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Repeat,
    DeleteEvents,
    Quantize,
    Init,
    DeleteTrack,
    Crop,
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
    if matches!(action, Action::Crop) {
        match crop(app, &selected) {
            Ok(start) => {
                app.event_selection = State::default();
                crate::cursor::moves::at_tick(app, start);
            }
            Err(error) => app.load_error = Some(error),
        }
        return;
    }
    if matches!(action, Action::DeleteEvents | Action::Quantize) {
        let result = app.edit_music(|music| {
            let track = &mut music.tracks[track];
            if matches!(action, Action::Quantize) {
                let grid = edit::quantize::sixteenth(music.division)?;
                edit::quantize::notes(track, &selected, grid)
            } else {
                edit::event_range::delete(track, &selected, notes)
            }
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
        Action::DeleteEvents | Action::Quantize => unreachable!(),
        Action::Init => edit::event_range::initialize(music, track),
        Action::DeleteTrack => {
            if music.tracks.len() == 1 {
                return Err("最後のSMF trackは削除できません".into());
            }
            music.tracks.remove(track);
            Ok(())
        }
        Action::Repeat | Action::Crop => unreachable!(),
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

/// Crops the edited pattern to the selection; returns the pattern's first tick in the editor.
fn crop(app: &mut App, selected: &[usize]) -> Result<u64, String> {
    let context = crate::view::patterns::mode::editing(app, app.track)
        .cloned()
        .ok_or("patternを開いてください")?;
    let l = app.loaded.as_ref().unwrap();
    let view = l.visible().clone();
    app.edit_tracks(|music| context.crop(music, &view, selected))?;
    context
        .edit_range(&app.loaded.as_ref().unwrap().music)
        .map(|r| r.0)
}
