//! Refresh edited music and seek without losing the running scope or A–B range.
use super::{App, Mode, Prepared};

pub(crate) struct Resume {
    tick: u64,
    end: u64,
    tracks: Vec<usize>,
    mode: Mode,
    clock: usize,
    pitches: Option<Vec<(usize, u8)>>,
}

pub(crate) fn running(app: &App) -> bool {
    app.playback.pending.is_some() || app.playback.active.is_some()
}

pub(crate) fn capture(app: &App) -> Option<Resume> {
    let (active, elapsed) = if let Some(p) = &app.playback.pending {
        (&p.active, 0.0)
    } else {
        (
            app.playback.active.as_ref()?,
            app.audition.seconds().unwrap_or(0.0),
        )
    };
    let map = active.tempos.get(active.clock_track)?;
    Some(Resume {
        tick: map.tick_after(0, active.start_seconds + elapsed + 1e-9),
        end: map.tick_after(0, active.start_seconds + active.end + 1e-9),
        tracks: active.sources.iter().map(|s| s.0).collect(),
        mode: active.mode,
        clock: active.clock_track,
        pitches: matches!(active.mode, Mode::Event | Mode::Notes).then(|| {
            active
                .sources
                .iter()
                .flat_map(|(track, s)| {
                    s.notes
                        .iter()
                        .filter(move |n| n.start <= elapsed && n.start + n.duration > elapsed)
                        .map(move |n| (*track, n.pitch))
                })
                .collect()
        }),
    })
}

pub(crate) fn prepare(
    app: &App,
    start: u64,
    end: u64,
    tracks: &[usize],
    mode: Mode,
    clock: usize,
) -> Option<Prepared> {
    if start >= end {
        return None;
    }
    let all = super::prepare(app, Mode::AllTracks)?;
    let map = all.active.tempos.get(clock)?;
    let offset = map.duration(0, start);
    let duration = map.duration(start, end - start);
    let mut prepared = crate::play::selection_playback::skip(app, all.active, offset)?;
    prepared.parts.retain(|p| tracks.contains(&p.0));
    for (_, _, seq) in &mut prepared.parts {
        seq.end = duration;
        seq.notes.retain_mut(|n| {
            n.duration = n.duration.min((duration - n.start).max(0.0));
            n.start < duration && n.duration > 0.0
        });
    }
    prepared.active.mode = mode;
    prepared.active.clock_track = clock;
    prepared.active.end = duration;
    prepared.active.sources = prepared
        .parts
        .iter()
        .map(|(t, _, s)| (*t, s.clone()))
        .collect();
    Some(prepared)
}

fn replace(app: &mut App, prepared: Option<Prepared>) {
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = prepared;
}

pub(crate) fn resume(app: &mut App, saved: Resume) {
    let mut prepared = prepare(
        app,
        saved.tick,
        saved.end,
        &saved.tracks,
        saved.mode,
        saved.clock,
    )
    .or_else(|| {
        app.playback
            .repeat
            .and_then(|r| super::repeat::prepare(app, r))
    });
    if let Some(pitches) = saved.pitches
        && let Some(prepared) = &mut prepared
    {
        for (track, _, sequence) in &mut prepared.parts {
            sequence
                .notes
                .retain(|n| n.start == 0.0 && pitches.contains(&(*track, n.pitch)));
        }
        prepared.active.sources = prepared
            .parts
            .iter()
            .map(|(t, _, s)| (*t, s.clone()))
            .collect();
    }
    replace(app, prepared);
}

fn end_at(app: &App, track: usize, tick: u64, mode: Mode, tracks: &[usize]) -> Option<u64> {
    let loaded = app.loaded.as_ref()?;
    let last = tracks
        .iter()
        .filter_map(|&t| loaded.music.tracks.get(t)?.events.last().map(|e| e.tick))
        .max()?;
    if !matches!(
        mode,
        Mode::Measure | Mode::BeatOnly | Mode::Event | Mode::Notes
    ) {
        return Some(last);
    }
    let map = loaded.time.map(track)?;
    let mut pos = map.position(tick);
    pos.tick_numerator = 0;
    pos.tick_denominator = 1;
    if mode == Mode::Measure {
        pos.measure += 1;
        pos.beat = 1;
    } else {
        pos.beat += 1;
    }
    Some(map.tick_at(&pos)?.min(last))
}

/// Playing clicks seek at the exact tick. Within A–B, keep B and the next loop;
/// outside A–B, leave the loop and resume the normal ending family there.
pub(crate) fn seek(app: &mut App, track: usize, tick: u64) {
    let Some(saved) = capture(app) else {
        return;
    };
    let end = if let Some((a, b)) = app.playback.repeat {
        if (a..b).contains(&tick) {
            Some(b)
        } else {
            app.playback.repeat = None;
            end_at(app, track, tick, saved.mode, &saved.tracks)
        }
    } else {
        end_at(app, track, tick, saved.mode, &saved.tracks)
    };
    let prepared = end.and_then(|end| prepare(app, tick, end, &saved.tracks, saved.mode, track));
    crate::cursor::split::go(app, track, tick);
    replace(app, prepared);
}

pub(crate) fn audition(app: &mut App, track: usize, tick: u64) {
    let tracks: Vec<_> = (0..app.loaded.as_ref().map_or(0, |l| l.music.tracks.len())).collect();
    let mode = match app.playback.last {
        Mode::Measure => Mode::Measure,
        Mode::BeatOnly | Mode::Event | Mode::Notes => Mode::BeatOnly,
        _ => Mode::AllTracks,
    };
    let prepared = end_at(app, track, tick, mode, &tracks)
        .and_then(|end| prepare(app, tick, end, &tracks, mode, track));
    replace(app, prepared);
}

#[cfg(test)]
mod tests;
