//! The one rule for which pattern a container shows in Notes; Raw shows the saved track.
//!
//! A container's track packs its bodies end to end, and that storage timeline means nothing as
//! music, so Notes never shows it: every view reads the projection of exactly one pattern.
//!
//! 1. While a Song or Scene runs, the pattern it plays now in the current container.
//! 2. Otherwise the one last shown in the container: a candidate or cell click, an edit, or the
//!    pattern rule 1 stopped at.
//! 3. Otherwise the container's first pattern.
use super::*;
use crate::play::playback::Mode;
use cat_smf_editor::score::{pattern::Kind, scene::arrangement};

/// Rule 1. Among patterns sounding together the shown one stays, so the list does not flip.
pub(super) fn sounding(a: &App) -> Option<u64> {
    let active = a.playback.active.as_ref()?;
    let Mode::Arrangement(scene) = active.mode else {
        return None;
    };
    let tick = crate::play::transport::tick_for(a, active.clock_track)?;
    let l = a.loaded.as_ref()?;
    let (catalog, collection) = l.arrangement.as_ref()?;
    let (occurrences, _) =
        arrangement::occurrences_in(&l.music, catalog, collection, scene).ok()?;
    let here: Vec<u64> = occurrences
        .into_iter()
        .filter_map(|(id, at)| {
            let p = catalog.patterns.get(&id).filter(|p| p.track == a.track)?;
            (tick.checked_sub(at)? < p.length().ok()?).then_some(id)
        })
        .collect();
    let shown = a.patterns.editing.as_ref().map(|c| c.pattern);
    here.iter()
        .find(|id| Some(**id) == shown)
        .or(here.first())
        .copied()
}

/// The pattern the rule gives for the current track, when that is a container with patterns.
pub(super) fn resolve(a: &App) -> Option<u64> {
    sounding(a).or_else(|| {
        let (catalog, _) = a.loaded.as_ref()?.arrangement.as_ref()?;
        let owner = catalog
            .tracks
            .iter()
            .find_map(|(&id, &t)| (t == a.track).then_some(id))?;
        a.patterns
            .navigation
            .last(owner)
            .filter(|id| {
                catalog
                    .patterns
                    .get(id)
                    .is_some_and(|p| p.container == owner)
            })
            .or_else(|| {
                catalog
                    .patterns
                    .values()
                    .filter(|p| p.container == owner)
                    // The keyboard's Capture only when the container has nothing else.
                    .min_by_key(|p| (p.kind == Kind::Capture, p.tick))
                    .map(|p| p.id)
            })
    })
}

/// Whatever dropped the projection, no frame paints the storage timeline. Each frame starts
/// here; a path that drops it while a frame paints calls this itself.
pub(super) fn ensure(a: &mut App) {
    if a.startup.pending
        || a.patterns.editing.is_some()
        || !mode::workspace_open(a)
        || !mode::actual_source(a, a.track)
    {
        return;
    }
    if let Some(id) = resolve(a)
        && let Err(error) = navigation::show(a, id)
    {
        a.load_error = Some(error);
    }
}

/// Rule 1 for a running Song or Scene. It waits while the root window is being typed or dragged
/// in, because those hold rows of the shown pattern. The candidate stays, or a placement made
/// during playback would put whatever happens to be sounding.
pub fn follow(a: &mut App, ctx: &eframe::egui::Context) {
    if !mode::workspace_open(a)
        || !mode::actual_source(a, a.track)
        || !super::super::commands::root_idle(a)
        || ctx.input(|i| i.pointer.any_down())
    {
        return;
    }
    let Some(id) = sounding(a) else { return };
    if a.patterns.editing.as_ref().is_some_and(|c| c.pattern == id) {
        return;
    }
    let grid = crate::view::drums::is_open(a);
    match navigation::show(a, id) {
        Ok(()) if grid => crate::view::drums::follow(a, id),
        Ok(()) => {}
        Err(error) => a.load_error = Some(error),
    }
}
