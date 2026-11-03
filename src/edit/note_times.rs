//! Moves the Note On and Note Off of several notes at once; every other event keeps its tick
//! and its order.
use crate::message;
use crate::{
    edit::{end_of_track, refresh_deltas, relocate::pairs},
    model::{Event, Track},
    score::note_view::{note_off, note_on},
};

/// Gives each Note On index of `notes` its new start and end tick. On error the track is
/// unchanged.
pub fn retime(track: &mut Track, notes: &[(usize, u64, u64)]) -> Result<(), String> {
    let events = &track.events;
    let before = pairs(events.iter());
    let mut ticks: Vec<Option<u64>> = vec![None; events.len()];
    for &(on, start, end) in notes {
        if !events.get(on).is_some_and(note_on) {
            return Err(message::NOT_A_NOTE_ON.into());
        }
        let off = before[on].ok_or(message::THIS_NOTE_HAS_NO_NOTE_OFF)?;
        if end <= start {
            return Err(message::THE_DURATION_MUST_BE_AT_LEAST_1_TICK.into());
        }
        ticks[on] = Some(start);
        ticks[off] = Some(end);
    }
    let tick = |i: usize| ticks[i].unwrap_or(events[i].tick);
    let mut moved: Vec<usize> = (0..events.len()).filter(|&i| ticks[i].is_some()).collect();
    // On one tick a moved Note Off ends before a Note On starts there.
    moved.sort_by_key(|&i| (tick(i), note_on(&events[i]), i));
    let mut moved = moved.into_iter().peekable();
    let mut stay: Vec<usize> = (0..events.len()).filter(|&i| ticks[i].is_none()).collect();
    let end = stay.pop_if(|&mut i| end_of_track(&events[i]));
    let mut order = Vec::with_capacity(events.len());
    for i in stay {
        let e = &events[i];
        while let Some(&m) = moved.peek() {
            let t = tick(m);
            if t < e.tick || (t == e.tick && note_off(&events[m]) && note_on(e)) {
                order.push(m);
                moved.next();
            } else {
                break;
            }
        }
        order.push(i);
    }
    order.extend(moved);
    order.extend(end);
    let after = pairs(order.iter().map(|&i| &events[i]));
    // Pairing follows event order, so crossing a same-key note would change other lengths.
    let unchanged = order
        .iter()
        .enumerate()
        .all(|(position, &original)| after[position].map(|i| order[i]) == before[original]);
    if !unchanged {
        return Err("動かすと同じ音高の別のnoteと重なります".into());
    }
    let mut result: Vec<Event> = order
        .iter()
        .map(|&i| Event {
            tick: tick(i),
            ..events[i].clone()
        })
        .collect();
    let last = result
        .iter()
        .filter(|e| !end_of_track(e))
        .map(|e| e.tick)
        .max()
        .unwrap_or(0);
    if let Some(eot) = result.last_mut().filter(|_| end.is_some()) {
        eot.tick = eot.tick.max(last);
    }
    let mut result = Track { events: result };
    refresh_deltas(&mut result)?;
    *track = result;
    Ok(())
}

#[cfg(test)]
mod tests;
