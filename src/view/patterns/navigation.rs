//! User navigation owns both the pattern projection and its editor presentation.
//! Low-level track rebinding and cursor restoration must not start user navigation.
use super::*;
use cat_smf_editor::score::{
    musical_time::MusicalTime,
    pattern::{editing::Context, Catalog},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct State {
    // Stable container IDs survive track insertions; this is document-local, not persisted.
    last: BTreeMap<u64, u64>,
}

/// Keep user intent across the intermediate track/cursor states of a bulk edit or Undo.
pub struct Snapshot {
    context: Option<Context>,
    drum: bool,
}

pub fn snapshot(a: &App) -> Snapshot {
    Snapshot {
        context: mode::editing(a, a.track).filter(|c| c.standalone).cloned(),
        drum: crate::view::drums::is_open(a),
    }
}

pub fn restore(a: &mut App, snapshot: Snapshot) {
    a.patterns.editing = snapshot.context;
    a.patterns.edit_song = a.song_id;
    refresh(a);
    if snapshot.drum && mode::editing(a, a.track).is_some() {
        present(a);
    }
    crate::view::drums::reconcile(a);
}

/// Every restored or newly entered body uses the same editor-selection policy.
pub fn present(a: &mut App) {
    let drum = mode::editing(a, a.track).is_some()
        && a.loaded
            .as_ref()
            .is_some_and(|l| l.patches[a.track].drums.is_some());
    if drum {
        crate::view::drums::display(a);
    } else {
        crate::view::drums::close(a);
    }
}

pub fn enter_notes(a: &mut App) {
    if mode::editing(a, a.track).is_some() {
        present(a);
    } else if mode::actual_source(a, a.track)
        && a.loaded
            .as_ref()
            .is_some_and(|l| l.patches[a.track].drums.is_some())
    {
        // Raw can be entered without a prior body; returning to Notes still uses
        // the same last-pattern/first-pattern policy as an ordinary track entry.
        track_selected(a);
    }
}

pub fn edit(a: &mut App, id: u64) -> Result<(), String> {
    // Resolve and validate the destination before touching the old editor or transport.
    let loaded = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let catalog = Catalog::read(&loaded.music)?;
    let pattern = catalog.patterns.get(&id).ok_or("Missing pattern")?;
    let container = pattern.container;
    let context = Context {
        track: pattern.track,
        measure: 1,
        pattern: id,
        placement: None,
        scene: None,
        standalone: true,
    };
    let projection = context.project(&loaded.music)?;
    let drum = loaded
        .patches
        .get(pattern.track)
        .is_some_and(|p| p.drums.is_some());
    if drum {
        cat_smf_editor::score::drums::tick(projection.division, 0)?;
    }
    let time = MusicalTime::new(&projection);
    let map = time.map(context.track).ok_or("Missing pattern meter")?;
    let tick = cat_smf_editor::score::same_measure::bounds(map, 1)?.0;

    crate::play::transport::stop(a);
    crate::view::drums::close(a);
    a.note_mode = true;
    a.patterns.raw_context = None;
    a.set_track(context.track);
    a.patterns.container = Some(container);
    a.patterns.selected = Some(id);
    a.patterns.navigation.last.insert(container, id);
    a.patterns.editing = Some(context);
    a.patterns.edit_song = a.song_id;
    a.patterns.workspace_open = true;
    a.generation = a.generation.wrapping_add(1);
    refresh(a);
    crate::cursor::moves::at_tick(a, tick);
    present(a);
    if crate::view::drums::is_open(a) {
        workspace::reset_windows(a);
        a.patterns.root_focus = true;
    }
    Ok(())
}

pub fn track_selected(a: &mut App) {
    if !a.note_mode || !mode::actual_source(a, a.track) {
        return;
    }
    let result = select_source(a);
    if let Err(error) = result {
        a.load_error = Some(error);
    }
}

fn select_source(a: &mut App) -> Result<(), String> {
    let l = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let c = Catalog::read(&l.music)?;
    let owner = c
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == a.track).then_some(id))
        .ok_or("Missing sound container")?;
    let selected = a
        .patterns
        .navigation
        .last
        .get(&owner)
        .copied()
        .filter(|id| c.patterns.get(id).is_some_and(|p| p.container == owner))
        .or_else(|| {
            c.patterns
                .values()
                .filter(|p| p.container == owner)
                .min_by_key(|p| p.tick)
                .map(|p| p.id)
        });
    let drum = l.patches.get(a.track).is_some_and(|p| p.drums.is_some());
    a.patterns.container = Some(owner);
    a.patterns.selected = selected;
    if drum && let Some(id) = selected {
        // Repeated notifications must not erase numeric input or restart the editor.
        if !crate::view::drums::is_open(a)
            || mode::editing(a, a.track).is_none_or(|c| c.pattern != id)
        {
            edit(a, id)?;
        }
    } else {
        a.patterns.workspace.song_cell.focus = true;
    }
    Ok(())
}

/// Dismissing a body is a real navigation back to the library, not a hidden editor.
pub fn browse(a: &mut App) {
    crate::view::drums::close(a);
    a.patterns.editing = None;
    a.patterns.raw_context = None;
    a.patterns.workspace.return_to = None;
    a.note_edit = None;
    a.note_edit_request = None;
    a.generation = a.generation.wrapping_add(1);
    refresh(a);
    crate::cursor::moves::at_tick(a, 0);
    a.patterns.root_focus = false;
    a.patterns.workspace.song_cell.choice = a
        .patterns
        .selected
        .map(cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern);
    a.patterns.workspace.song_cell.placing = false;
    a.patterns.workspace.song_cell.focus = true;
}

#[cfg(test)]
mod tests;
