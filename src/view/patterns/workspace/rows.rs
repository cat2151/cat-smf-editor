//! Shown rows past the saved lanes stay empty until their first placement.
use super::*;
pub fn lanes(c: &scene::Collection, id: Option<u64>) -> Vec<u64> {
    id.map_or_else(
        || c.song_lanes.clone(),
        |id| {
            c.scenes
                .iter()
                .find(|s| s.id == id)
                .map_or_else(Vec::new, |s| s.lanes.clone())
        },
    )
}
fn occupied(c: &scene::Collection, id: Option<u64>, lane: u64) -> bool {
    match id {
        Some(id) => c
            .scenes
            .iter()
            .find(|s| s.id == id)
            .is_some_and(|s| s.cells.iter().any(|p| p.lane == lane)),
        None => c.song.iter().any(|p| p.lane == lane),
    }
}
/// Four rows, and four more once the last block of four is used.
pub fn rows(c: &scene::Collection, id: Option<u64>) -> usize {
    let lanes = lanes(c, id);
    let used = lanes
        .iter()
        .rposition(|&l| occupied(c, id, l))
        .map_or(0, |i| (i + 1) / 4);
    lanes.len().max(4 * (used + 1))
}
pub fn lane(a: &App, p: Pane) -> Option<u64> {
    let c = scene::read(&a.loaded.as_ref()?.music).ok()?;
    lanes(&c, scene_id(a, p))
        .get(a.patterns.workspace.selection(p).row)
        .copied()
}
pub fn materialize(
    m: &mut cat_smf_editor::model::Music,
    id: Option<u64>,
    row: usize,
) -> Result<u64, String> {
    loop {
        if let Some(&lane) = lanes(&scene::read(m)?, id).get(row) {
            return Ok(lane);
        }
        model::add_lane(m, id)?;
    }
}
