//! Row changes use ordinary chain persistence and one atomic undo snapshot.
use super::*;
pub(super) fn add(a: &mut App) -> Result<(), String> {
    let loaded = a.loaded.as_ref().ok_or("No song")?;
    let mut patch = loaded.patches[a.track].clone();
    let kit = patch.kit.as_mut().ok_or("No kit")?;
    let mut rows = drum_kit::layout::rows(&loaded.music.tracks[a.track], kit);
    if rows.len() > 124 {
        return Err("Maximum 128 rows".into());
    }
    rows.splice(0..0, [None; 4]);
    kit.rows = Some(rows);
    crate::player::selection::store_track(a, a.track, &patch)?;
    a.drum_editor.kit.slot = 0;
    a.drum_editor.kit.note = None;
    a.drum_editor.kit.query.clear();
    a.drum_editor.kit.query_before = None;
    a.drum_editor.kit.reveal = true;
    sync(a);
    Ok(())
}
pub(super) fn assign(a: &mut App, note: u8) -> Result<(), String> {
    let (slot, field, page) = (
        a.drum_editor.kit.slot,
        a.drum_editor.field,
        a.drum_editor.page,
    );
    let loaded = a.loaded.as_ref().ok_or("No song")?;
    let mut music = loaded.music.clone();
    let mut patch = loaded.patches[a.track].clone();
    drum_kit::layout::assign(
        &mut music.tracks[a.track],
        patch.kit.as_mut().ok_or("No kit")?,
        a.drum_editor.kit.slot,
        note,
    )?;
    cat_smf_editor::score::instrument::write(
        &mut music.tracks[a.track],
        &patch,
        &mut a.patch_roots.lookup(a.player_selection.executable.clone()),
    )?;
    cat_smf_editor::edit::regenerate(&mut music)?;
    a.audition.stop_key();
    a.drum_editor.kit.preview = None;
    crate::player::selection::store_track_with(a, a.track, &patch, |m| {
        *m = music;
        Ok(())
    })?;
    a.drum_editor.kit.slot = slot;
    a.drum_editor.field = field;
    a.drum_editor.page = page;
    a.drum_editor.kit.note = Some(note);
    a.drum_editor.kit.query.clear();
    a.drum_editor.kit.query_before = None;
    a.drum_editor.kit.reveal = true;
    sync(a);
    Ok(())
}
