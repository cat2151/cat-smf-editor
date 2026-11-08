//! User navigation owns both the pattern projection and its editor presentation.
//! Low-level track rebinding and cursor restoration must not start user navigation.
use super::*;
use cat_smf_editor::message;
use cat_smf_editor::score::{
    musical_time::MusicalTime,
    pattern::{editing::Context, Catalog},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
pub struct State {
    // Stable container IDs survive track insertions; this is document-local, not persisted.
    last: BTreeMap<u64, u64>,
    /// Ordinary containers last edited on the melody step grid; the session keeps them.
    pub(crate) steps: BTreeSet<u64>,
}

/// Keep user intent across the intermediate track/cursor states of a bulk edit or Undo.
pub struct Snapshot {
    context: Option<Context>,
    drum: bool,
}

impl State {
    pub(super) fn last(&self, container: u64) -> Option<u64> {
        self.last.get(&container).copied()
    }
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

/// A Drum or Drum kit container, or an ordinary one last edited on the step grid.
fn grid(a: &App, container: u64) -> bool {
    a.patterns.navigation.steps.contains(&container)
        || a.loaded
            .as_ref()
            .is_some_and(|l| l.patches.get(a.track).is_some_and(|p| p.sequencer()))
}

/// The melody grid was opened (`true`) or dismissed for the edited pattern's container.
pub fn remember_grid(a: &mut App, open: bool) {
    if let Some(container) = a.patterns.container {
        let steps = &mut a.patterns.navigation.steps;
        if open {
            steps.insert(container);
        } else {
            steps.remove(&container);
        }
    }
}

/// Every restored or newly entered body uses the same editor-selection policy.
pub fn present(a: &mut App) {
    let drum =
        mode::editing(a, a.track).is_some() && a.patterns.container.is_some_and(|c| grid(a, c));
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
            .is_some_and(|l| l.patches[a.track].sequencer())
    {
        // Raw can be entered without a prior body; returning to Notes still uses
        // the same last-pattern/first-pattern policy as an ordinary track entry.
        track_selected(a);
    } else {
        shown::ensure(a);
    }
}

/// `id`'s context, its container and first tick, checked before anything changes.
fn destination(a: &App, id: u64) -> Result<(Context, u64, u64), String> {
    let loaded = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let catalog = Catalog::read(&loaded.music)?;
    let pattern = catalog.patterns.get(&id).ok_or(message::MISSING_PATTERN)?;
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
        .is_some_and(|p| p.sequencer());
    if drum {
        cat_smf_editor::score::drums::tick(projection.division, 0)?;
    }
    let time = MusicalTime::new(&projection);
    let map = time
        .map(context.track)
        .ok_or(message::MISSING_PATTERN_METER)?;
    let tick = cat_smf_editor::score::same_measure::bounds(map, 1)?.0;
    Ok((context, container, tick))
}

/// The projection of `context`, leaving the transport, the candidate and every editor alone.
fn install(a: &mut App, context: Context, container: u64, tick: u64) {
    a.patterns.timing.pattern = None;
    let pattern = context.pattern;
    a.patterns.container = Some(container);
    a.patterns.selected = Some(context.pattern);
    a.patterns
        .navigation
        .last
        .insert(container, context.pattern);
    a.patterns.editing = Some(context);
    a.patterns.edit_song = a.song_id;
    a.generation = a.generation.wrapping_add(1);
    refresh(a);
    crate::cursor::moves::at_tick(a, tick);
    crate::view::recording::follow_pattern(a, pattern);
}

/// Shows `id`, a pattern of the current track, as `shown` decides it.
pub(super) fn show(a: &mut App, id: u64) -> Result<(), String> {
    let (context, container, tick) = destination(a, id)?;
    install(a, context, container, tick);
    Ok(())
}

/// The grid takes the window from the arrangement panes it was opened from.
fn present_body(a: &mut App) {
    present(a);
    if crate::view::drums::is_open(a) {
        workspace::reset_windows(a);
        a.patterns.root_focus = true;
        crate::view::drums::take_keys(a);
    }
}

pub fn edit(a: &mut App, id: u64) -> Result<(), String> {
    let (context, container, tick) = destination(a, id)?;
    if !a.startup.restoring {
        crate::play::transport::stop(a);
    }
    crate::view::drums::close(a);
    a.note_mode = true;
    a.patterns.raw_context = None;
    a.set_track(context.track);
    a.patterns.workspace.song_cell.choice =
        Some(cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern(id));
    a.patterns.workspace.song_cell.reveal_choice = true;
    install(a, context, container, tick);
    present_body(a);
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
    let l = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let c = Catalog::read(&l.music)?;
    let owner = c
        .tracks
        .iter()
        .find_map(|(&id, &t)| (t == a.track).then_some(id))
        .ok_or(message::MISSING_SOUND_CONTAINER)?;
    let selected = shown::resolve(a);
    let drum = grid(a, owner);
    a.patterns.container = Some(owner);
    a.patterns.selected = selected;
    // Repeated notifications must not erase numeric input, restart the editor or move the cursor.
    if let Some(id) = selected
        && mode::editing(a, a.track).is_none_or(|c| c.pattern != id)
    {
        show(a, id)?;
    }
    if !drum || selected.is_none() {
        a.patterns.workspace.song_cell.focus = true;
    } else if !crate::view::drums::is_open(a) {
        present_body(a);
    }
    Ok(())
}

/// Dismissing a body is a real navigation back to the library, not a hidden editor;
/// `shown` puts the pattern back on the event list.
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
    shown::ensure(a);
}

#[cfg(test)]
mod tests;
