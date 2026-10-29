//! Tempo view's transient all-track A–B range, in conductor ticks.
use super::{App, Mode, Prepared};

pub fn prepare(app: &App, (a, b): (u64, u64)) -> Option<Prepared> {
    if let Some(mode) = app
        .playback
        .repeat_mode
        .filter(|_| app.playback.repeat.is_some())
    {
        return prepare_in(app, (a, b), mode);
    }
    if a >= b {
        return None;
    }
    if let Some((tracks, clock)) = crate::view::drums::playback::scope(app, (a, b)) {
        return super::timeline::prepare(app, a, b, &tracks, Mode::AllTracks, clock);
    }
    let all = super::prepare(app, Mode::AllTracks)?;
    let tempo = all.active.tempos.first()?;
    let start = tempo.duration(0, a);
    let duration = tempo.duration(a, b - a);
    let mut prepared = crate::play::selection_playback::skip(app, all.active, start)?;
    for (_, _, sequence) in &mut prepared.parts {
        sequence.truncate(duration);
    }
    prepared.active.end = duration;
    prepared.active.clock_track = 0;
    prepared.active.sources = prepared
        .parts
        .iter()
        .map(|(track, _, sequence)| (*track, sequence.clone()))
        .collect();
    crate::view::parameter_editor::preview::apply(app, &mut prepared);
    Some(prepared)
}

pub fn start(app: &mut App, range: (u64, u64)) {
    app.playback.repeat_mode = None;
    if play_once(app, range) {
        app.playback.repeat = Some(range);
    }
}

/// Plays `range` without looping; false when it has nothing to play.
pub fn play_once(app: &mut App, range: (u64, u64)) -> bool {
    app.playback.repeat_mode = None;
    let Some(prepared) = prepare(app, range) else {
        return false;
    };
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = Some(prepared);
    app.playback.repeat = None;
    true
}

pub fn clear(app: &mut App) {
    if app.playback.repeat.take().is_none() {
        return;
    }
    let seconds = app
        .playback
        .pending
        .as_ref()
        .map(|p| p.active.start_seconds)
        .or_else(|| {
            app.playback
                .active
                .as_ref()
                .map(|a| a.start_seconds + app.audition.seconds().unwrap_or(0.0))
        });
    if let Some(seconds) = seconds {
        let mode = app.playback.repeat_mode.take().unwrap_or(Mode::AllTracks);
        let pending = super::prepare(app, mode)
            .and_then(|p| crate::play::selection_playback::skip(app, p.active, seconds));
        app.audition.stop();
        app.playback.active = None;
        app.playback.pending = pending;
    }
}

/// A Song range uses its arrangement even while a candidate pattern is open.
pub(crate) fn start_song(app: &mut App, range: (u64, u64)) {
    let mode = Mode::Arrangement(None);
    let Some(prepared) = prepare_in(app, range, mode) else {
        return;
    };
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = Some(prepared);
    app.playback.repeat = Some(range);
    app.playback.repeat_mode = Some(mode);
}

fn prepare_in(app: &App, (a, b): (u64, u64), mode: Mode) -> Option<Prepared> {
    if a >= b {
        return None;
    }
    let all = super::prepare(app, mode)?;
    let tempo = all.active.tempos.first()?;
    super::super::arrangement::remaining(app, None, tempo.seconds_at(a), tempo.duration(a, b - a))
}
