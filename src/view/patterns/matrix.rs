use super::*;
/// User entry to pattern work: an empty library needs a body before arranging.
/// Keep this policy shared by the style chooser and command palette.
pub fn enter(a: &mut App, ctx: &eframe::egui::Context) -> Result<(), String> {
    let music = &a.loaded.as_ref().ok_or("No SMF is loaded")?.music;
    if cat_smf_editor::score::pattern::Catalog::read(music)?
        .patterns
        .is_empty()
    {
        library::begin_add(a, ctx)
    } else {
        open(a)
    }
}

// Raw window opening is internal to patterns; other UI must use `enter`.
pub(super) fn open(a: &mut App) -> Result<(), String> {
    if a.loaded.is_none() {
        return Err("No SMF is loaded".into());
    }
    workspace::enable(a);
    a.patterns.workspace.song_cell.focus = true;
    Ok(())
}
