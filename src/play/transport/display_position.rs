//! Playback on the visible pattern timeline, independent of the Song window's selection.
use super::{Active, App, Mode};
use crate::view::patterns::mode;

pub(super) fn clock_track(app: &App, track: usize, active: &Active) -> usize {
    if mode::editing(app, track).is_some()
        && matches!(active.mode, Mode::Pattern(_) | Mode::Arrangement(_))
    {
        active.clock_track
    } else {
        track
    }
}

pub(crate) fn tick_for(app: &App, track: usize) -> Option<u64> {
    let active = app.playback.active.as_ref()?;
    let tick = super::tick_for(app, clock_track(app, track, active))?;
    project(app, track, active, tick)
}

pub(super) fn project(app: &App, track: usize, active: &Active, tick: u64) -> Option<u64> {
    let Some(context) = mode::editing(app, track) else {
        return Some(tick);
    };
    match active.mode {
        Mode::Pattern(id) => pattern_tick(app, track, id, tick),
        Mode::Arrangement(scene) => {
            let loaded = app.loaded.as_ref()?;
            let (catalog, collection) = loaded.arrangement.as_ref()?;
            let occurrences = cat_smf_editor::score::scene::arrangement::occurrences_in(
                &loaded.music,
                catalog,
                collection,
                scene,
            )
            .ok()?
            .0;
            occurrences
                .into_iter()
                .find_map(|(id, at)| pattern_tick(app, track, id, tick.checked_sub(at)?))
        }
        _ => {
            // Direct playback (including the grid's own loop) already uses display ticks.
            let (start, end) = app.loaded.as_ref()?.edit_range(context).ok()?;
            (start..end).contains(&tick).then_some(tick)
        }
    }
}

pub(crate) fn pattern_tick(app: &App, track: usize, id: u64, local: u64) -> Option<u64> {
    let context = mode::editing(app, track).filter(|c| c.pattern == id)?;
    let (start, end) = app.loaded.as_ref()?.edit_range(context).ok()?;
    (local < end - start).then_some(start + local)
}
