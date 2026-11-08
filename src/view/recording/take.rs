//! A take committed to the song and opened where the keyboard opened from.
use super::*;
use cat_smf_editor::message;

pub(super) fn finish(a: &mut App, s: &Session) -> Result<(), String> {
    let recorder = s.recorder.as_ref().ok_or("録音が始まっていません")?;
    let source = source(a, s.sounds[s.selected].container)?;
    let mut created = vec![];
    a.edit_tracks(|music| {
        created = recorder.commit(music, source, s.sweep_start())?;
        Ok(())
    })?;
    let id = *created.last().ok_or("録音したpatternが見つかりません")?;
    reveal(a, s, id)
}

/// The track of `container`; `None` for the fallback sound of a song without a container.
pub(super) fn source(a: &App, container: Option<u64>) -> Result<Option<usize>, String> {
    let Some(container) = container else {
        return Ok(None);
    };
    Catalog::read(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?
        .tracks
        .get(&container)
        .copied()
        .map(Some)
        .ok_or("録音先のcontainerが無くなりました".into())
}

/// After the keyboard's own edit `id` created the song's first container, the fallback sound
/// plays that container.
pub(super) fn adopt(a: &App, s: &mut Session, id: u64) -> Result<(), String> {
    let c = Catalog::read(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?;
    let container = c.patterns[&id].container;
    for sound in s.sounds.iter_mut().filter(|s| s.container.is_none()) {
        sound.container = Some(container);
        sound.track = c.tracks[&container];
    }
    Ok(())
}

/// Selects a new take where the keyboard opened from: the pattern windows, or the editor.
pub(super) fn reveal(a: &mut App, s: &Session, id: u64) -> Result<(), String> {
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music)?;
    a.patterns.selected = Some(id);
    a.patterns.container = Some(catalog.patterns[&id].container);
    if let Some(p) = [Pane::Scene, Pane::Song]
        .into_iter()
        .find(|p| p.view() == s.from)
    {
        a.patterns.workspace.selection_mut(p).choice = Some(PatternOrScene::Pattern(id));
        a.patterns.workspace.selection_mut(p).query.clear();
    } else {
        super::super::patterns::library::edit(a, id)?;
    }
    Ok(())
}
