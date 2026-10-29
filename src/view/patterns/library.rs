//! Independent pattern editing uses the ordinary note tools without a song placement.
use super::*;
use cat_smf_editor::score::pattern::{operations, Catalog};

/// With several containers the target is chosen each time, starting from the current one.
pub fn add(a: &mut App, ctx: &eframe::egui::Context) {
    if let Err(error) = begin_add(a, ctx) {
        a.load_error = Some(error);
    }
}
pub(crate) fn begin_add(a: &mut App, ctx: &eframe::egui::Context) -> Result<(), String> {
    let several = first::choices(a).is_ok_and(|c| c.len() > 1);
    if several {
        first::open(a)?;
    } else {
        create(a, None)?;
    }
    input(a, ctx);
    Ok(())
}
/// A new body starts in the same input as `i`.
pub(crate) fn input(a: &mut App, ctx: &eframe::egui::Context) {
    presentation::focus_root(a, ctx);
    if !a.patterns.confirming_creation() {
        crate::view::drums::input(a, ctx);
    }
}

pub fn create(a: &mut App, duplicate: Option<u64>) -> Result<(), String> {
    let catalog = Catalog::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
    let source = duplicate
        .and_then(|id| catalog.patterns.get(&id).map(|p| p.track))
        .or_else(|| {
            a.patterns
                .container
                .and_then(|id| catalog.tracks.get(&id).copied())
                .filter(|t| catalog.sources.contains(t))
        })
        .or_else(|| catalog.sources.contains(&a.track).then_some(a.track));
    let Some(source) = source else {
        return first::open(a);
    };
    create_at(a, source, duplicate)
}
pub(crate) fn create_for_container(a: &mut App, id: u64) -> Result<(), String> {
    let catalog = Catalog::read(&a.loaded.as_ref().ok_or("No SMF is loaded")?.music)?;
    let source = catalog
        .tracks
        .get(&id)
        .copied()
        .filter(|t| catalog.sources.contains(t))
        .ok_or("Missing sound container")?;
    create_at(a, source, None)?;
    a.patterns.container = Some(id);
    Ok(())
}
fn create_at(a: &mut App, source: usize, duplicate: Option<u64>) -> Result<(), String> {
    let mut created = None;
    a.edit_tracks(|music| {
        created = Some(operations::create(music, source, duplicate)?);
        Ok(())
    })?;
    edit(a, created.ok_or("Missing created pattern")?)
}
/// Prepare both initial sounds once, only after the creation choice was confirmed.
pub(super) fn initialize(a: &mut App, drums: bool) -> Result<(), String> {
    let (patch, _) = crate::view::drums::prepare(a)?;
    initialize_with(a, drums, patch)
}
pub(super) fn initialize_with(
    a: &mut App,
    drums: bool,
    patch: cat_smf_editor::score::instrument::Patch,
) -> Result<(), String> {
    let mut music = a.loaded.as_ref().ok_or("No SMF is loaded")?.music.clone();
    let saw = source(a, &mut music, None)?;
    let drum = source(a, &mut music, Some(&patch))?;
    commit(a, music, if drums { drum } else { saw })
}
/// The container and its first pattern are one undo step.
pub(super) fn add_container(a: &mut App, drums: bool) -> Result<(), String> {
    let patch = if drums {
        Some(crate::view::drums::prepare(a)?.0)
    } else {
        None
    };
    add_container_with(a, patch)
}
pub(super) fn add_container_with(
    a: &mut App,
    drums: Option<cat_smf_editor::score::instrument::Patch>,
) -> Result<(), String> {
    let mut music = a.loaded.as_ref().ok_or("No SMF is loaded")?.music.clone();
    let track = source(a, &mut music, drums.as_ref())?;
    commit(a, music, track)
}
/// Same-kind containers get numbered names so the chooser can tell them apart.
fn source(
    a: &mut App,
    music: &mut cat_smf_editor::model::Music,
    drums: Option<&cat_smf_editor::score::instrument::Patch>,
) -> Result<usize, String> {
    let base = if drums.is_some() { "Drums" } else { "Init Saw" };
    let taken = |n: &str| music.tracks.iter().any(|t| t.name().as_deref() == Some(n));
    let name = std::iter::once(base.to_owned())
        .chain((2..).map(|i| format!("{base} {i}")))
        .find(|n| !taken(n))
        .unwrap();
    let track = operations::create_source(music)?;
    music.tracks[track]
        .events
        .iter_mut()
        .find(|e| e.status == 0xff && e.meta_type == 3)
        .unwrap()
        .data = name.into_bytes();
    if let Some(patch) = drums {
        cat_smf_editor::score::instrument::write(
            &mut music.tracks[track],
            patch,
            &mut a.patch_roots.lookup(a.player_selection.executable.clone()),
        )?;
    }
    Ok(track)
}
fn commit(
    a: &mut App,
    mut music: cat_smf_editor::model::Music,
    track: usize,
) -> Result<(), String> {
    let id = operations::create(&mut music, track, None)?;
    a.edit_tracks(|m| {
        *m = music;
        Ok(())
    })?;
    edit(a, id)
}

pub use super::navigation::edit;
