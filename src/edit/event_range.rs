//! Event-list edits preserve the absolute ticks of surviving events.
use crate::model::{Event, Music, Track};

pub fn delete(track: &mut Track, selected: &[usize], notes: bool) -> Result<(), String> {
    let mut remove = vec![false; track.events.len()];
    for &i in selected {
        *remove.get_mut(i).ok_or("Invalid event selection")? = true;
    }
    if notes {
        for (on, off) in super::relocate::pairs(track.events.iter())
            .into_iter()
            .enumerate()
        {
            if remove[on]
                && let Some(off) = off
            {
                remove[off] = true;
            }
        }
    }
    let mut i = 0;
    track.events.retain(|_| {
        let keep = !remove[i];
        i += 1;
        keep
    });
    finish(track)
}

/// Reset to the whole-event intersection of all other tracks' initial events.
pub fn initialize(music: &mut Music, target: usize) -> Result<(), String> {
    if target >= music.tracks.len() {
        return Err("Invalid track".into());
    }
    let events = super::tracks::common_initial(
        music
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, track)| (i != target).then_some(track)),
    );
    let track = &mut music.tracks[target];
    track.events = events;
    finish(track)
}

fn finish(track: &mut Track) -> Result<(), String> {
    if !track.events.last().is_some_and(super::end_of_track) {
        track.events.push(Event {
            delta: 0,
            tick: track.events.last().map_or(0, |e| e.tick),
            status: 0xff,
            meta_type: 0x2f,
            data: vec![],
        });
    }
    super::refresh_deltas(track)
}

#[cfg(test)]
mod tests;
