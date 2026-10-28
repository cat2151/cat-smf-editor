use crate::{
    app::App,
    play::playback::{self, Mode},
};
use cat_smf_editor::{score::audition_timing::TempoMap, score::display_rows::DisplayRow};
use eframe::egui;

pub struct Active {
    pub automation: super::automation::Automation,
    pub mode: Mode,
    pub clock_track: usize,
    pub tempos: Vec<TempoMap>,
    pub start_seconds: f64,
    pub end: f64,
    pub sources: Vec<(usize, crate::play::audio_sequence::Sequence)>,
}

pub struct Prepared {
    pub parts: Vec<(
        usize,
        cat_smf_editor::score::instrument::Patch,
        crate::play::audio_sequence::Sequence,
    )>,
    pub active: Active,
    /// Pattern, Scene and Song playback: what each container track sounds, before any skip.
    /// `None` plays the tracks as stored.
    pub origin: Option<Origin>,
}

/// The expanded events each track sounds, timed by `tempo` from second 0.
pub struct Origin {
    pub events: std::collections::BTreeMap<usize, Vec<cat_smf_editor::model::Event>>,
    pub tempo: TempoMap,
}

// Capture the source and range at the start request, independent of later view changes.
pub fn prepare(app: &App, mode: Mode) -> Option<Prepared> {
    if let Mode::Pattern(id) = mode {
        return crate::view::patterns::preview::prepare(app, id).ok();
    }
    if let Mode::Arrangement(scene) = mode {
        return super::arrangement::prepare(app, scene).ok();
    }
    let standalone = app.patterns.editing.as_ref().is_some_and(|c| c.standalone);
    if standalone
        && matches!(
            mode,
            Mode::Track | Mode::AllTracks | Mode::AllTracksFromMeasure
        )
    {
        return crate::view::patterns::preview::prepare(
            app,
            app.patterns.editing.as_ref()?.pattern,
        )
        .ok();
    }
    let _span = cat_smf_editor::startup_trace::span("sequence_prepare");
    if mode == Mode::AllTracksFromMeasure {
        let all = prepare(app, Mode::AllTracks)?;
        let seconds = all.active.tempos[app.track].duration(0, start_tick(app, Mode::MeasureStart));
        let mut prepared = crate::play::selection_playback::skip(app, all.active, seconds)?;
        if !matches!(prepared.active.mode, Mode::Arrangement(_)) {
            prepared.active.mode = mode;
        }
        return Some(prepared);
    }
    let loaded = app.loaded.as_ref()?;
    if mode == Mode::AllTracks
        && !cat_smf_editor::score::scene::read(&loaded.music)
            .ok()?
            .song
            .is_empty()
    {
        return super::arrangement::prepare(app, None).ok();
    }
    let parts = playback::parts(app, mode);
    let end = parts.iter().map(|(_, _, s)| s.end).fold(0.0, f64::max);
    let start = start_tick(app, mode);
    let local_tempo = standalone
        .then(|| TempoMap::constant(loaded.music.division, app.patterns.tempo.0))
        .transpose()
        .ok()?;
    let tempos: Vec<_> = (0..loaded.music.tracks.len())
        .map(|t| {
            local_tempo
                .clone()
                .unwrap_or_else(|| TempoMap::new(&loaded.music, t))
        })
        .collect();
    let start_seconds = tempos.get(app.track)?.duration(0, start);
    let sources = parts
        .iter()
        .map(|(track, _, sequence)| (*track, sequence.clone()))
        .collect();
    let mut automation =
        super::automation::Automation::prepare(app, &parts, mode == Mode::AllTracks);
    if standalone {
        automation =
            crate::view::patterns::preview::prepare(app, app.patterns.editing.as_ref()?.pattern)
                .ok()?
                .active
                .automation;
    }
    if let Err(error) = cat_smf_editor::score::same_measure::validate(&loaded.music) {
        automation.error = Some(error);
    }
    Some(Prepared {
        parts,
        active: Active {
            automation,
            mode,
            clock_track: app.track,
            tempos,
            start_seconds,
            end,
            sources,
        },
        origin: None,
    })
}

fn start_tick(app: &App, mode: Mode) -> u64 {
    if matches!(mode, Mode::Track | Mode::AllTracks) {
        return 0;
    }
    let (start, position) = playback::location(app);
    if matches!(
        mode,
        Mode::Measure | Mode::MeasureStart | Mode::Beat | Mode::BeatOnly
    ) && let Some(mut position) = position
    {
        position.tick_numerator = 0;
        position.tick_denominator = 1;
        if !matches!(mode, Mode::Beat | Mode::BeatOnly) {
            position.beat = 1;
        }
        return app
            .loaded
            .as_ref()
            .and_then(|l| l.time.map(app.track))
            .and_then(|m| m.tick_at(&position))
            .unwrap_or(start);
    }
    start
}

pub(crate) mod timeline;

/// Restarts running playback in its mode from `tick`; the cursor must already be there.
pub fn jump(app: &mut App, tick: u64) {
    if app.playback.active.is_none() && app.playback.pending.is_none() {
        return;
    }
    let mode = app
        .playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(app.playback.active.as_ref())
        .unwrap()
        .mode;
    let Some(prepared) = prepare(app, mode) else {
        return;
    };
    // These modes start at tick 0 whatever the cursor; the others start at the cursor.
    let prepared = if matches!(mode, Mode::Track | Mode::AllTracks | Mode::Pattern(_)) {
        let seconds = prepared.active.tempos[app.track].duration(0, tick);
        crate::play::selection_playback::skip(app, prepared.active, seconds)
    } else {
        Some(prepared)
    };
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = prepared;
}

// Prepare the audio device after the start request's frame can be painted.
pub fn advance(app: &mut App) {
    crate::view::drums::playback::validate(app);
    crate::view::macro_editor::validate(app);
    crate::view::parameter_editor::preview::validate(app);
    if let Some(prepared) = app.playback.pending.take() {
        if let Some(loaded) = &app.loaded
            && let Err(error) = cat_smf_editor::score::same_measure::validate(&loaded.music)
        {
            stop(app);
            app.load_error = Some(error);
            return;
        }
        if let Some(error) = &prepared.active.automation.error {
            app.audition.stop();
            app.playback.active = None;
            app.playback.repeat = None;
            app.load_error = Some(format!("Automation: {error}"));
            return;
        }
        match crate::play::sung::prerender::takes(app, &prepared) {
            // Still preparing: the singer's WAV is being made in the background.
            None => app.playback.pending = Some(prepared),
            Some(takes) => {
                let started = app.audition.sequences_automated(
                    prepared.parts,
                    takes,
                    &prepared.active.automation,
                    prepared.active.start_seconds,
                );
                if started && prepared.active.end > 0.0 {
                    app.playback.active = Some(prepared.active);
                } else if !started {
                    app.playback.repeat = None;
                }
            }
        }
    }
    if app.playback.active.is_some() && app.audition.sequence_failed() {
        app.playback.active = None;
        app.playback.repeat = None;
        return;
    }
    if let Some(active) = &app.playback.active
        && app.audition.seconds().is_none_or(|s| s >= active.end)
    {
        app.playback.active = None;
        if let Some(range) = app.playback.repeat {
            app.playback.pending = repeat::prepare(app, range);
        }
    }
}

pub fn tick(app: &App) -> Option<u64> {
    tick_for(app, app.track)
}

pub fn tick_for(app: &App, track: usize) -> Option<u64> {
    let active = app.playback.active.as_ref()?;
    let seconds = app.audition.seconds().filter(|s| *s > 0.0)?;
    Some(
        active
            .tempos
            .get(track)?
            .tick_after(0, active.start_seconds + seconds),
    )
}

/// True from the start request until the audio callback renders its first frame:
/// plugins open on the UI thread and the stream may not have run yet.
pub fn preparing(app: &App) -> bool {
    app.playback.pending.is_some()
        || app.playback.active.is_some() && app.audition.seconds() == Some(0.0)
}

/// Row where playback will start while it is still preparing.
pub fn preparing_row_for(app: &App, track: usize) -> Option<usize> {
    if !preparing(app) {
        return None;
    }
    let active = app
        .playback
        .pending
        .as_ref()
        .map(|p| &p.active)
        .or(app.playback.active.as_ref())?;
    let tick = active
        .tempos
        .get(track)?
        .tick_after(0, active.start_seconds);
    // Rows sharing the start tick: the overlay stays on the cursor, not the tick's last row.
    if track == app.track
        && let Some(cursor) = app.cursor_row
        && crate::cursor::split::row_count(app, track) > cursor
        && crate::cursor::split::row_tick(app, track, cursor) == tick
    {
        return Some(cursor);
    }
    row_for_tick(app, track, tick)
}

#[cfg(test)]
pub fn marker_row(app: &App) -> Option<usize> {
    marker_for(app, app.track)
}

pub fn marker_for(app: &App, track: usize) -> Option<usize> {
    row_for_tick(app, track, tick_for(app, track)?)
}

/// Last row at or before `tick`.
fn row_for_tick(app: &App, track: usize, tick: u64) -> Option<usize> {
    let loaded = app.loaded.as_ref()?;
    let events = &loaded.visible().tracks[track].events;
    let count = if app.note_mode {
        loaded.display_rows[track].len()
    } else {
        events.len()
    };
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = low + (high - low) / 2;
        let row_tick = if app.note_mode {
            match loaded.display_rows[track].get(mid) {
                DisplayRow::Event(i) => events[i].tick,
                DisplayRow::Beat(p) => loaded.time.map(track)?.tick_at(&p)?,
            }
        } else {
            events[mid].tick
        };
        if row_tick <= tick {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    (count > 0).then(|| low.saturating_sub(1))
}

/// Stop before preparing audio or dispatching keys to overlays.
pub fn stop_shortcut(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route_stop(app, ctx);
}

pub fn stop(app: &mut App) {
    app.patterns.preview = None;
    let preview = app.playback.parameter_preview.take().is_some();
    app.playback.pending = None;
    app.playback.active = None;
    app.playback.repeat = None;
    app.audition.stop();
    if preview {
        app.audition.restore_automated_baselines();
    }
    if app.cursor_row.is_none() && crate::cursor::split::row_count(app, app.track) > 0 {
        let tick = playback::location(app).0;
        app.cursor_row = Some(crate::cursor::split::row_sounding_loaded(
            app.loaded.as_ref().unwrap(),
            app.note_mode,
            app.track,
            tick,
        ));
        app.track_view.cursor_tick = Some(tick);
    }
    app.reveal_cursor = true;
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
}

mod status;
pub use status::show as status;

#[cfg(test)]
mod jump_tests;
#[cfg(test)]
mod stop_tests;
#[cfg(test)]
mod tests;

pub(crate) mod repeat;
