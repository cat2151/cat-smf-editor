//! 読み込んだ SMF への編集。どの編集も触れたイベント以外の tick を変えない。
pub mod event_range;
pub mod mml;
pub mod note_length;
pub mod note_move;
pub mod note_start;
pub mod reference_tempo;
mod relocate;
pub mod tempo_curve;
pub mod tracks;
pub mod trill;

/// Rebuild derived notes and then the tempo events at their final positions.
pub fn regenerate(music: &mut crate::model::Music) -> Result<(), String> {
    let curve = tempo_curve::read(music)?;
    // Stage the whole operation so malformed metadata or unrepresentable speeds
    // never leave a partially changed song.
    let mut next = music.clone();
    trill::rebuild(&mut next, curve.as_ref())?;
    if let Some(curve) = &curve {
        tempo_curve::regenerate(&mut next, curve)?;
    }
    *music = next;
    Ok(())
}

use crate::model::{Event, Track};

fn end_of_track(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 0x2f
}

/// Places `event` after the events already at its tick, keeping End of Track last.
pub fn insert(track: &mut Track, event: Event) {
    let end = track.events.pop_if(|e| end_of_track(e));
    let at = track.events.partition_point(|e| e.tick <= event.tick);
    track.events.insert(at, event);
    if let Some(mut end) = end {
        end.tick = end.tick.max(track.events.last().map_or(0, |e| e.tick));
        track.events.push(end);
    }
}

/// Rewrites delta from tick so the raw view matches what saving writes.
pub fn refresh_deltas(track: &mut Track) -> Result<(), String> {
    let mut previous = 0;
    for event in &mut track.events {
        let delta = event
            .tick
            .checked_sub(previous)
            .ok_or("SMF events are out of tick order")?;
        event.delta = u32::try_from(delta).map_err(|_| "Delta time is too large")?;
        previous = event.tick;
    }
    Ok(())
}
