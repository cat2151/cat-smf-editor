//! Moves one event to another tick; every other event keeps its tick.
use crate::{
    edit::{end_of_track, refresh_deltas},
    model::{Event, Track},
    score::note_view::{note_off, note_on},
};
use std::collections::{HashMap, VecDeque};

/// Note Off index for each Note On, paired first-in first-out per channel and key like NoteView.
pub(crate) fn pairs<'a>(events: impl Iterator<Item = &'a Event>) -> Vec<Option<usize>> {
    let events: Vec<_> = events.collect();
    let mut pending: HashMap<(u8, u8), VecDeque<usize>> = HashMap::new();
    let mut result = vec![None; events.len()];
    for (index, event) in events.iter().enumerate() {
        let Some(&key) = event.data.first() else {
            continue;
        };
        let key = (event.status & 15, key);
        if note_on(event) {
            pending.entry(key).or_default().push_back(index);
        } else if note_off(event)
            && let Some(on) = pending.get_mut(&key).and_then(VecDeque::pop_front)
        {
            result[on] = Some(index);
        }
    }
    result
}

/// Places `index` after the events already at `tick`, keeping End of Track last.
/// The same tick keeps the event where it is.
pub(super) fn relocate(track: &mut Track, index: usize, tick: u64) -> Result<(), String> {
    let events = &track.events;
    if events[index].tick == tick {
        return Ok(());
    }
    let before = pairs(events.iter());
    let mut order: Vec<usize> = (0..events.len()).filter(|&i| i != index).collect();
    let end = order.pop_if(|&mut i| end_of_track(&events[i]));
    let at = order.partition_point(|&i| events[i].tick <= tick);
    order.insert(at, index);
    order.extend(end);
    let after = pairs(order.iter().map(|&i| &events[i]));
    // Pairing follows event order, so crossing a same-key note would change other lengths.
    let unchanged = order
        .iter()
        .enumerate()
        .all(|(position, &original)| after[position].map(|i| order[i]) == before[original]);
    if !unchanged {
        return Err("The move crosses another note of the same key".into());
    }
    let mut moved: Vec<Event> = order.iter().map(|&i| events[i].clone()).collect();
    moved[at].tick = tick;
    if let Some(last) = moved.last_mut().filter(|_| end.is_some()) {
        last.tick = last.tick.max(tick);
    }
    let mut result = Track { events: moved };
    refresh_deltas(&mut result)?;
    *track = result;
    Ok(())
}
