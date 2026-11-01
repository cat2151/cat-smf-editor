//! A take committed to the song and opened where the keyboard opened from.
use super::*;
use cat_smf_editor::message;

pub(super) fn finish(a: &mut App, s: &Session) -> Result<(), String> {
    let recorder = s.recorder.as_ref().ok_or("録音が始まっていません")?;
    let sound = &s.sounds[s.selected];
    let source = if let Some(container) = sound.container {
        Some(
            *Catalog::read(&a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?
                .tracks
                .get(&container)
                .ok_or("録音先のcontainerが無くなりました")?,
        )
    } else {
        None
    };
    let mut created = vec![];
    a.edit_tracks(|music| {
        created = recorder.commit(music, source)?;
        Ok(())
    })?;
    let id = *created.last().ok_or("録音したpatternが見つかりません")?;
    reveal(a, s, id, recorder.settings().tempo)
}

/// Selects a new take where the keyboard opened from: the pattern windows, or the editor.
pub(super) fn reveal(a: &mut App, s: &Session, id: u64, tempo: u32) -> Result<(), String> {
    // Candidates audition at the pattern preview tempo, so takes replay as recorded.
    a.patterns.tempo.0 = 60_000_000.0 / f64::from(tempo);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music)?;
    a.patterns.selected = Some(id);
    a.patterns.container = Some(catalog.patterns[&id].container);
    if let Some(p) = [Pane::Scene, Pane::Song]
        .into_iter()
        .find(|p| p.viewport() == s.origin)
    {
        a.patterns.workspace.selection_mut(p).choice = Some(PatternOrScene::Pattern(id));
        a.patterns.workspace.selection_mut(p).query.clear();
    } else {
        super::super::patterns::library::edit(a, id)?;
    }
    Ok(())
}
