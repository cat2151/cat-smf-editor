//! Measure navigation and the drum editor's temporary audition scope.
use super::*;
use cat_smf_editor::message;

pub(super) fn measure(app: &App) -> Result<drums::Measure, String> {
    let loaded = app.loaded.as_ref().ok_or(message::NO_SONG)?;
    let result = drums::Measure::new(
        loaded
            .time
            .map(app.track)
            .ok_or(message::MISSING_MEASURE_MAP)?,
        loaded.music.division,
        app.drum_editor.page + 1,
    )?;
    if let Some(context) = crate::view::patterns::mode::editing(app, app.track) {
        let range = loaded.edit_range(context)?;
        if result.start < range.0 || result.end > range.1 {
            return Err("小節がpatternの長さの外です".into());
        }
    }
    Ok(result)
}

/// Step of `measure` now sounding, from the transport or else from a repeating phrase.
pub(super) fn sounding(app: &App, measure: &drums::Measure) -> Option<usize> {
    let tick =
        crate::play::transport::display_tick_for(app, app.track).or_else(|| phrase_tick(app))?;
    if !(measure.start..measure.end).contains(&tick) {
        return None;
    }
    measure.steps.iter().rposition(|&s| s <= tick)
}

/// The tick a Phrase loop repeating the edited pattern plays now.
fn phrase_tick(app: &App) -> Option<u64> {
    let id = crate::view::patterns::mode::editing(app, app.track)?.pattern;
    crate::play::transport::pattern_tick(
        app,
        app.track,
        id,
        crate::view::recording::phrase_tick(app, id)?,
    )
}

pub(super) fn range(app: &App) -> Result<(u64, u64), String> {
    let loaded = app.loaded.as_ref().ok_or(message::NO_SONG)?;
    let map = loaded
        .time
        .map(app.track)
        .ok_or(message::MISSING_MEASURE_MAP)?;
    let s = &app.drum_editor;
    let result = (
        drums::Measure::new(map, loaded.music.division, s.first)?.start,
        drums::Measure::new(map, loaded.music.division, s.last)?.end,
    );
    if let Some(context) = crate::view::patterns::mode::editing(app, app.track) {
        let range = loaded.edit_range(context)?;
        if result.0 < range.0 || result.1 > range.1 {
            return Err("drumの範囲がpatternの長さの外です".into());
        }
    }
    Ok(result)
}

/// Solo chooses among the song's tracks; an edited pattern loops alone.
pub(super) fn soloable(app: &App) -> bool {
    app.drum_editor.loop_enabled && !app.patterns.editing.as_ref().is_some_and(|c| c.standalone)
}

pub(crate) fn scope(app: &App, requested: (u64, u64)) -> Option<(Vec<usize>, usize)> {
    let s = &app.drum_editor;
    if !is_open(app) || !s.loop_enabled || s.confirm_create || range(app).ok()? != requested {
        return None;
    }
    let loaded = app.loaded.as_ref()?;
    let tracks = if s.all_tracks {
        (0..loaded.music.tracks.len()).collect()
    } else {
        vec![app.track]
    };
    Some((tracks, app.track))
}

pub(super) fn restart(app: &mut App) -> Result<(), String> {
    if !app.drum_editor.loop_enabled {
        return Ok(());
    }
    let range = range(app)?;
    // The grid's range is the edited pattern's, not the Song's an earlier A-B repeat left.
    app.playback.repeat_mode = None;
    let prepared =
        crate::play::transport::repeat::prepare(app, range).ok_or("drumのloopを準備できません")?;
    app.audition.stop();
    app.playback.active = None;
    app.playback.pending = Some(prepared);
    app.playback.repeat = Some(range);
    app.drum_editor.owned_range = Some(range);
    Ok(())
}

pub(super) fn page(app: &mut App, next: bool) -> Result<(), String> {
    let s = &mut app.drum_editor;
    let before = s.page;
    s.page = if next {
        s.page.saturating_add(1)
    } else {
        s.page.saturating_sub(1)
    };
    let m = match measure(app) {
        Ok(m) => m,
        Err(e) => {
            app.drum_editor.page = before;
            return Err(e);
        }
    };
    let s = &mut app.drum_editor;
    s.field = s.field.min(m.steps.len() + 1);
    s.first = m.number;
    s.last = m.number;
    restart(app)
}

pub(super) fn boundary(app: &mut App, end: bool, delta: i32) -> Result<(), String> {
    let before = (app.drum_editor.first, app.drum_editor.last);
    let s = &mut app.drum_editor;
    if end {
        s.last = s.last.saturating_add_signed(i64::from(delta)).max(s.first);
    } else {
        s.first = s
            .first
            .saturating_add_signed(i64::from(delta))
            .max(1)
            .min(s.last);
    }
    if let Err(e) = range(app) {
        (app.drum_editor.first, app.drum_editor.last) = before;
        return Err(e);
    }
    restart(app)
}

pub(super) fn close(app: &mut App) {
    if app.drum_editor.owned_range.is_some() && app.playback.repeat == app.drum_editor.owned_range {
        crate::play::transport::stop(app);
    }
    app.drum_editor.owned_range = None;
}

pub(crate) fn validate(app: &mut App) {
    if !is_open(app) {
        super::close(app);
    }
}
