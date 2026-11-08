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
            // An ordinary container's pattern shows the melody grid.
            crate::view::patterns::mode::editing(app, app.track).is_some_and(|c| c.pattern == id)
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
        if super::melody::active(app) {
            return Ok(());
        }
        ensure(app)
    });
    if let Err(error) = prepare {
        app.load_error = Some(error);
        return;
    }
    if !is_open(app) {
        display(app);
    }
    if super::melody::active(app) {
        crate::view::patterns::navigation::remember_grid(app, true);
    }
    take_keys(app);
}

/// A grid opened by the user's own input takes the keys of the pane it sits in. Undo and other
/// re-presentations leave them where they are.
pub(crate) fn take_keys(app: &mut App) {
    use crate::view::dock::{focus, ViewKind};
    if is_open(app) && focus::placed(app, ViewKind::StepSequencer) {
        focus::set(app, ViewKind::StepSequencer);
    }
}

/// Present an already resolved target. No creation, SMF writes or audition here.
pub(crate) fn display(app: &mut App) {
    if app.drum_editor.kit.take_preview() {
        app.audition.stop_key();
    }
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
    let melody = super::melody::active(app);
    // Undo and other re-presentations of the same pattern keep the pitch and step.
    let kept = matches!((app.drum_editor.target, target), (Some(Target::Pattern(a)), Target::Pattern(b)) if a == b)
        && melody;
    let (field, melody_state) = if kept {
        (
            app.drum_editor.field,
            std::mem::take(&mut app.drum_editor.melody),
        )
    } else {
        (2, Default::default())
    };
    playback::close(app);
    app.drum_editor = State {
        target: Some(target),
        confirm_create: !melody && needs_track(app),
        field,
        melody: melody_state,
        page: number.saturating_sub(1),
        first: number,
        last: number,
        catalog: app.drum_editor.catalog.take(),
        ..Default::default()
    };
    // Floating editors give way to the grid. Placed views keep their panes; `focus::set` hands
    // the keys among them, so an Undo leaves the keys where they are.
    use crate::view::dock::{focus, ViewKind};
    let floats = |view| !focus::placed(app, view);
    let (piano, tempo, curve, mixer) = (
        floats(ViewKind::PianoRoll),
        floats(ViewKind::TempoCurve),
        floats(ViewKind::ParameterCurve),
        floats(ViewKind::Mixer),
    );
    app.layer_editor.open = false;
    app.piano_roll.open &= !piano;
    app.tempo_curve.open &= !tempo;
    if curve {
        app.parameter_editor.close();
    }
    app.effect_editor = Default::default();
    app.playback.open = false;
    app.playback.mixer_open &= !mixer;
    app.comma_menu.open = false;
    app.edit_menu.open = false;
    app.track_view.go_open = false;
    app.track_view.split_open = false;
    app.note_edit = None;
    app.note_edit_request = None;
    app.trill_edit = None;
}

/// The grid moves with a running Song; `display` would close the panels open beside it.
pub(crate) fn follow(app: &mut App, id: u64) {
    if app.drum_editor.kit.take_preview() {
        app.audition.stop_key();
    }
    app.drum_editor.target = Some(Target::Pattern(id));
    app.drum_editor.page = 0;
    app.drum_editor.first = 1;
    app.drum_editor.last = 1;
}

pub(crate) fn close(app: &mut App) {
    if app.drum_editor.kit.take_preview() {
        app.audition.stop_key();
    }
    playback::close(app);
    app.drum_editor = State::default();
}

pub(crate) fn dismiss(app: &mut App) {
    if super::melody::active(app) {
        crate::view::patterns::navigation::remember_grid(app, false);
    }
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
