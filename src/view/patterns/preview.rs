use crate::{
    app::App,
    play::{playback::Mode, transport::Prepared},
};
use cat_smf_editor::message;
use cat_smf_editor::score::{instrument::Patch, pattern::Catalog};

/// Single-pattern audition shares the arrangement expansion and container voice plan.
pub fn prepare(a: &App, id: u64) -> Result<Prepared, String> {
    let loaded = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let c = Catalog::read(&loaded.music)?;
    let p = c
        .patterns
        .get(&id)
        .ok_or("試聴するpatternが見つかりません")?;
    let tempo = super::timing::pattern(a, id)?;
    crate::play::arrangement::prepare_occurrences(
        a,
        &[(id, 0)],
        p.length()?,
        tempo,
        Mode::Pattern(id),
        p.track,
    )
}
pub fn remaining(a: &App, id: u64, start: f64, duration: f64) -> Option<Prepared> {
    let mut p = prepare(a, id).ok()?;
    let duration = duration.min(p.active.end - start);
    if start < 0.0 || duration <= 0.0 {
        return None;
    }
    for (_, _, s) in &mut p.parts {
        s.skip(start);
        s.truncate(duration);
    }
    p.active.start_seconds = start;
    p.active.end = duration;
    p.active.sources = p.parts.iter().map(|(t, _, s)| (*t, s.clone())).collect();
    Some(p)
}
pub fn sound(a: &App, id: u64) -> Result<(usize, Patch), String> {
    let loaded = a.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?;
    let music = &loaded.music;
    let c = Catalog::read(music)?;
    let p = c.patterns.get(&id).ok_or(message::MISSING_PATTERN)?;
    let track = p.track;
    let patch = loaded
        .patches
        .get(track)
        .ok_or(message::MISSING_CONTAINER_SOUND)?
        .clone();
    Ok((track, patch))
}
/// Ordinary cursor/MML audition in independent editing uses the same sound as preview.
pub fn edit_sound(a: &App, track: usize) -> Option<(usize, Patch)> {
    let edit = a
        .patterns
        .editing
        .as_ref()
        .filter(|c| c.standalone && c.track == track)?;
    sound(a, edit.pattern).ok()
}
pub fn start(a: &mut App, id: u64) -> Result<(), String> {
    crate::play::transport::stop(a);
    let tick = super::timing::pattern_tick(a, id)?;
    let prepared = prepare(a, id)?;
    a.patterns.timing.playing = Some((
        cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern(id),
        tick,
    ));
    a.playback.last = Mode::Pattern(id);
    a.playback.follow = false;
    a.playback.pending = Some(prepared);
    a.patterns.preview = Some(id);
    Ok(())
}
