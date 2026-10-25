//! Measure navigation and the drum editor's temporary audition scope.
use super::*;

pub(super) fn measure(app: &App) -> Result<drums::Measure, String> {
    let loaded = app.loaded.as_ref().ok_or("No song")?;
    drums::Measure::new(
        loaded.time.map(app.track).ok_or("Missing measure map")?,
        loaded.music.division,
        app.drum_editor.page + 1,
    )
}

fn range(app: &App) -> Result<(u64, u64), String> {
    let loaded = app.loaded.as_ref().ok_or("No song")?;
    let map = loaded.time.map(app.track).ok_or("Missing measure map")?;
    let s = &app.drum_editor;
    Ok((
        drums::Measure::new(map, loaded.music.division, s.first)?.start,
        drums::Measure::new(map, loaded.music.division, s.last)?.end,
    ))
}

pub(crate) fn scope(app: &App, requested: (u64, u64)) -> Option<(Vec<usize>, usize)> {
    let s = &app.drum_editor;
    if !s.open || !s.loop_enabled || s.confirm_create || range(app).ok()? != requested {
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
    let prepared =
        crate::play::transport::repeat::prepare(app, range).ok_or("Cannot prepare drum loop")?;
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
    if !app.drum_editor.open {
        close(app);
    }
}
