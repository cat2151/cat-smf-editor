//! A drum surface is bound to its real edit target, never an independent open flag.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Target {
    Track(usize),
    Pattern(u64),
}

pub(crate) fn is_open(app: &App) -> bool {
    let Some(l) = &app.loaded else { return false };
    if app.track >= l.music.tracks.len() {
        return false;
    }
    match app.drum_editor.target {
        Some(Target::Track(track)) => track == app.track,
        Some(Target::Pattern(id)) => {
            crate::view::patterns::mode::editing(app, app.track).is_some_and(|c| c.pattern == id)
                && l.patches.get(app.track).is_some_and(|p| p.drums.is_some())
        }
        None => false,
    }
}

/// Explicit drum input can create a new body/track. Display alone cannot do so.
pub(crate) fn open(app: &mut App) {
    if app.loaded.is_none() || super::super::parameter_editor::is_track(app) {
        return;
    }
    let prepare = pattern::edit(app).and_then(|()| {
        drums::tick(app.loaded.as_ref().unwrap().music.division, 0)?;
        ensure(app)
    });
    if let Err(error) = prepare {
        app.load_error = Some(error);
        return;
    }
    if !is_open(app) {
        display(app);
    }
}

/// Present an already resolved target. No creation, SMF writes or audition here.
pub(crate) fn display(app: &mut App) {
    let target = crate::view::patterns::mode::editing(app, app.track)
        .map_or(Target::Track(app.track), |c| Target::Pattern(c.pattern));
    let tick = crate::play::playback::location(app).0;
    let number = app
        .loaded
        .as_ref()
        .and_then(|l| l.time.map(app.track))
        .map_or(1, |map| {
            u64::try_from(map.position(tick).measure).unwrap_or(1)
        });
    playback::close(app);
    app.drum_editor = State {
        target: Some(target),
        confirm_create: needs_track(app),
        field: 2,
        page: number.saturating_sub(1),
        first: number,
        last: number,
        catalog: app.drum_editor.catalog.take(),
        ..Default::default()
    };
    app.layer_editor.open = false;
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.parameter_editor.close();
    app.effect_editor = Default::default();
    app.playback.open = false;
    app.playback.mixer_open = false;
    app.comma_menu.open = false;
    app.edit_menu.open = false;
    app.track_view.go_open = false;
    app.track_view.split_open = false;
    app.note_edit = None;
    app.note_edit_request = None;
    app.trill_edit = None;
}

pub(crate) fn close(app: &mut App) {
    playback::close(app);
    app.drum_editor = State::default();
}

pub(crate) fn dismiss(app: &mut App) {
    if matches!(app.drum_editor.target, Some(Target::Pattern(_))) {
        crate::view::patterns::navigation::browse(app);
    } else {
        close(app);
    }
}

/// Rebinding/undo can remove a pattern or shorten it while an input is pending.
pub(crate) fn reconcile(app: &mut App) {
    if app.drum_editor.target.is_none() {
        return;
    }
    if !is_open(app) {
        let pattern = matches!(app.drum_editor.target, Some(Target::Pattern(_)));
        close(app);
        if pattern && app.note_mode {
            app.patterns.workspace.song_cell.focus = true;
        }
    } else if matches!(app.drum_editor.target, Some(Target::Pattern(_)))
        && (playback::measure(app).is_err() || playback::range(app).is_err())
    {
        // A resized/replaced body starts at its first measure without old input or loops.
        crate::cursor::moves::at_tick(app, 0);
        display(app);
    }
}
