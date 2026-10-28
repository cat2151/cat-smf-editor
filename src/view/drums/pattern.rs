//! Open the grid against a body, never the container's packed storage timeline.
use crate::{app::App, view::patterns};
use cat_smf_editor::score::pattern::Catalog;

pub(crate) fn input(a: &mut App, ctx: &eframe::egui::Context) {
    if patterns::mode::actual_source(a, a.track)
        && a.loaded
            .as_ref()
            .is_some_and(|l| l.patches[a.track].drums.is_some())
    {
        super::open(a);
    } else {
        crate::view::busy::request(a, ctx, crate::view::busy::Task::MmlInput);
    }
}

pub(super) fn edit(a: &mut App) -> Result<(), String> {
    if !a.note_mode || !patterns::mode::actual_source(a, a.track) {
        return Ok(());
    }
    if patterns::mode::editing(a, a.track).is_some() {
        return Ok(());
    }
    let l = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    if l.patches[a.track].drums.is_none() {
        return Err("Select a drum sound container".into());
    }
    let c = Catalog::read(&l.music)?;
    let owner = c
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == a.track).then_some(id))
        .ok_or("Missing drum container")?;
    a.patterns.container = Some(owner);
    let selected = a
        .patterns
        .editing
        .as_ref()
        .filter(|edit| edit.track == a.track)
        .map(|edit| edit.pattern)
        .or(a.patterns.selected)
        .filter(|id| c.patterns.get(id).is_some_and(|p| p.container == owner))
        .or_else(|| {
            c.patterns
                .values()
                .filter(|p| p.container == owner)
                .min_by_key(|p| p.tick)
                .map(|p| p.id)
        });
    if let Some(id) = selected {
        patterns::library::edit(a, id)
    } else {
        patterns::library::create(a, None)
    }
}
