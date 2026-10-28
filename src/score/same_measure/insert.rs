use super::*;

/// Validate the whole batch before exposing any changes to the caller.
pub fn insert(
    music: &mut Music,
    track: usize,
    first: u64,
    last: u64,
    destination: u64,
) -> Result<(), String> {
    if crate::score::pattern::is_source(music, track) {
        return Err("Select a normal track".into());
    }
    if first == 0 || last < first || destination == 0 {
        return Err("Invalid measure range".into());
    }
    let count = last - first;
    let final_destination = destination.checked_add(count).ok_or("Measure overflow")?;
    if first <= final_destination && destination <= last {
        return Err("Source and destination overlap".into());
    }
    let mut next = music.clone();
    let time = MusicalTime::new(music);
    let map = time.map(track).ok_or("Same measure requires PPQN timing")?;
    let target = next.tracks.get_mut(track).ok_or("Missing track")?;
    for offset in 0..=count {
        let tick = bounds(map, u128::from(destination + offset))?.0;
        crate::edit::insert(target, event(tick, first + offset));
    }
    crate::edit::refresh_deltas(target)?;
    validate(&next)?;
    *music = next;
    Ok(())
}
