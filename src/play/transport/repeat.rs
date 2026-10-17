//! Tempo view's transient all-track A–B range, in conductor ticks.
use super::{App, Mode, Prepared};

pub fn prepare(app: &App, (a, b): (u64, u64)) -> Option<Prepared> {
    if a >= b {
        return None;
    }
    let all = super::prepare(app, Mode::AllTracks)?;
    let tempo = all.active.tempos.first()?;
    let start = tempo.duration(0, a);
    let duration = tempo.duration(a, b - a);
    let mut prepared = crate::play::selection_playback::skip(app, all.active, start)?;
    for (_, _, sequence) in &mut prepared.parts {
        sequence.end = duration;
        sequence.notes.retain_mut(|note| {
            note.duration = note.duration.min((duration - note.start).max(0.0));
            note.start < duration && note.duration > 0.0
        });
    }
    prepared.active.end = duration;
    prepared.active.clock_track = 0;
    prepared.active.sources = prepared
        .parts
        .iter()
        .map(|(track, _, sequence)| (*track, sequence.clone()))
        .collect();
    Some(prepared)
}

pub fn start(app: &mut App, range: (u64, u64)) {
    let Some(prepared) = prepare(app, range) else {
        return;
    };
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = Some(prepared);
    app.playback.repeat = Some(range);
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
        let pending = super::prepare(app, Mode::AllTracks)
            .and_then(|p| crate::play::selection_playback::skip(app, p.active, seconds));
        app.audition.stop();
        app.playback.active = None;
        app.playback.pending = pending;
    }
}
