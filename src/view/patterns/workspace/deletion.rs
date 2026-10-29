//! Remove a Song candidate and all its occurrences as one undoable edit.
use super::*;

pub(super) fn pattern(a: &mut App, id: u64) -> Result<(), String> {
    mutate(a, |music| {
        let mut collection = scene::read(music)?;
        collection
            .song
            .retain(|p| p.part != PatternOrScene::Pattern(id));
        for scene in &mut collection.scenes {
            scene.cells.retain(|cell| cell.pattern != id);
        }
        scene::write(music, &collection)?;
        cat_smf_editor::score::pattern::operations::delete(music, id)
    })?;
    if a.patterns.selected == Some(id) {
        a.patterns.selected = None;
    }
    for p in [Pane::Scene, Pane::Song] {
        let s = a.patterns.workspace.selection_mut(p);
        if s.choice == Some(PatternOrScene::Pattern(id)) {
            s.choice = None;
        }
    }
    Ok(())
}
