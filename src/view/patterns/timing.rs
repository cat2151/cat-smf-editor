//! Auditions use conductor time at a placement, or at the Song cursor.
use super::workspace::Pane;
use crate::app::App;
use cat_smf_editor::{
    message,
    score::{audition_timing::TempoMap, scene::arrangement},
};

#[derive(Default)]
pub struct State {
    pub pattern: Option<PatternOrigin>,
    /// Scene ID, Song placement ID. None means the Scene was opened from candidates.
    pub scene: Option<(u64, u64)>,
    /// A running audition keeps its origin when the Song cursor is moved.
    pub playing: Option<(arrangement::PatternOrScene, u64)>,
}

pub struct PatternOrigin {
    pattern: u64,
    scene: Option<u64>,
    placement: u64,
    scene_placement: Option<u64>,
}

pub fn cursor(a: &App) -> Result<u64, String> {
    let music = &a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    arrangement::measure_tick(music, None, a.patterns.workspace.song_cell.measure)
}

pub fn cursor_map(a: &App) -> Option<TempoMap> {
    at(a, cursor(a).ok()?).ok()
}

pub fn bpm(a: &App) -> f64 {
    a.loaded
        .as_ref()
        .and_then(|l| {
            cursor_map(a)?
                .beat_clock(0.0, l.music.division)
                .map(|(_, hz)| hz * 60.0)
        })
        .unwrap_or(120.0)
}

pub fn scene_tick(a: &App, id: u64) -> Result<u64, String> {
    if let Some((arrangement::PatternOrScene::Scene(scene), tick)) = a.patterns.timing.playing
        && scene == id
    {
        return Ok(tick);
    }
    scene_origin(
        a,
        id,
        a.patterns.timing.scene.filter(|s| s.0 == id).map(|s| s.1),
    )
}

fn scene_origin(a: &App, id: u64, placement: Option<u64>) -> Result<u64, String> {
    let music = &a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    if let Some(placement) = placement {
        let collection = cat_smf_editor::score::scene::read(music)?;
        if let Some(p) = collection
            .song
            .iter()
            .find(|p| p.id == placement && p.part == arrangement::PatternOrScene::Scene(id))
        {
            return arrangement::measure_tick(music, None, p.measure);
        }
    }
    cursor(a)
}

pub fn pattern_tick(a: &App, id: u64) -> Result<u64, String> {
    if let Some((arrangement::PatternOrScene::Pattern(pattern), tick)) = a.patterns.timing.playing
        && pattern == id
    {
        return Ok(tick);
    }
    let music = &a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    if let Some(origin) = &a.patterns.timing.pattern
        && origin.pattern == id
    {
        let c = cat_smf_editor::score::scene::read(music)?;
        if let Some(scene) = origin.scene {
            if let Some(s) = c.scenes.iter().find(|s| s.id == scene)
                && let Some(cell) = s
                    .cells
                    .iter()
                    .find(|c| c.id == origin.placement && c.pattern == id)
            {
                return scene_origin(a, scene, origin.scene_placement)?
                    .checked_add(s.range(u128::from(cell.measure) + 1)?.0)
                    .ok_or_else(|| "試聴位置が上限を超えました".into());
            }
        } else if let Some(p) = c.song.iter().find(|p| {
            p.id == origin.placement && p.part == arrangement::PatternOrScene::Pattern(id)
        }) {
            return arrangement::measure_tick(music, None, p.measure);
        }
    }
    cursor(a)
}

pub fn pattern(a: &App, id: u64) -> Result<TempoMap, String> {
    at(a, pattern_tick(a, id)?)
}

pub fn scene(a: &App, id: u64) -> Result<TempoMap, String> {
    at(a, scene_tick(a, id)?)
}

fn at(a: &App, tick: u64) -> Result<TempoMap, String> {
    let music = &a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    Ok(TempoMap::new(music, 0).from_tick(tick))
}

/// Record which cell opened the editor, including a continuation cell's owner.
pub fn bind_pattern(a: &mut App, pane: Pane, id: u64, placement: u64) {
    let scene = super::workspace::scene_id(a, pane);
    let scene_placement = a
        .patterns
        .timing
        .scene
        .filter(|s| Some(s.0) == scene)
        .map(|s| s.1);
    a.patterns.timing.pattern = Some(PatternOrigin {
        pattern: id,
        scene,
        placement,
        scene_placement,
    });
}

/// An overview row identifies both the Song placement and the optional Scene cell.
pub(crate) fn bind_occurrence(
    a: &mut App,
    pattern: u64,
    placement: u64,
    scene: Option<(u64, u64)>,
) {
    a.patterns.timing.pattern = Some(PatternOrigin {
        pattern,
        scene: scene.map(|s| s.0),
        placement: scene.map_or(placement, |s| s.1),
        scene_placement: scene.map(|_| placement),
    });
}

pub fn bind_scene(a: &mut App, id: u64) {
    let collection = a
        .loaded
        .as_ref()
        .and_then(|l| l.arrangement.as_ref())
        .map(|c| &c.1);
    let s = &a.patterns.workspace.song_cell;
    a.patterns.timing.scene = collection.and_then(|c| {
        let lane = c.song_lanes.get(s.row)?;
        let (_, start) = arrangement::covering(c, None, *lane, s.measure)?;
        c.song
            .iter()
            .find(|p| {
                p.lane == *lane
                    && p.measure == start
                    && p.part == arrangement::PatternOrScene::Scene(id)
            })
            .map(|p| (id, p.id))
    });
}

#[cfg(test)]
mod tests;
