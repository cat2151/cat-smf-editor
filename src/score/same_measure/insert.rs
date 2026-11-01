use super::*;
use crate::message;

/// Validate the whole batch before exposing any changes to the caller.
pub fn insert(
    music: &mut Music,
    track: usize,
    first: u64,
    last: u64,
    destination: u64,
) -> Result<(), String> {
    if crate::score::pattern::is_source(music, track) {
        return Err("通常のtrackを選んでください".into());
    }
    if first == 0 || last < first || destination == 0 {
        return Err("小節の範囲が不正です".into());
    }
    let count = last - first;
    let final_destination = destination
        .checked_add(count)
        .ok_or(message::MEASURE_OVERFLOW)?;
    if first <= final_destination && destination <= last {
        return Err("移動元と移動先が重なっています".into());
    }
    let mut next = music.clone();
    let time = MusicalTime::new(music);
    let map = time
        .map(track)
        .ok_or(message::SAME_MEASURE_REQUIRES_PPQN_TIMING)?;
    let target = next.tracks.get_mut(track).ok_or(message::MISSING_TRACK)?;
    for offset in 0..=count {
        let tick = bounds(map, u128::from(destination + offset))?.0;
        crate::edit::insert(target, event(tick, first + offset));
    }
    crate::edit::refresh_deltas(target)?;
    validate(&next)?;
    *music = next;
    Ok(())
}
