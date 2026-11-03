//! Quantize of what the keys recorded, by the rule of [`crate::edit::quantize::note`].
use super::note;
use crate::model::Event;
use crate::score::note_view::{note_off, note_on};

/// The notes of `events` (ticks from the take's start) quantized to `grid`, the other events
/// kept. In a take of `length` ticks, which repeats, a note starting on its end starts at 0
/// instead. A note never starts before the previous note of its pitch ends.
pub(super) fn events(events: Vec<Event>, grid: u64, length: Option<u64>) -> Vec<Event> {
    let mut sounding: [Option<(u64, u8)>; 128] = [None; 128];
    let mut notes = vec![];
    let mut out = vec![];
    for e in events {
        let p = usize::from(e.data.first().copied().unwrap_or(0) & 0x7f);
        if note_on(&e) {
            sounding[p] = Some((e.tick, e.data[1]));
        } else if note_off(&e) {
            if let Some((start, velocity)) = sounding[p].take() {
                notes.push((start, e.tick, e.data[0], velocity));
            }
        } else {
            out.push(e);
        }
    }
    let mut placed: Vec<_> = notes
        .into_iter()
        .map(|(start, end, pitch, velocity)| {
            let (mut start, mut end) = crate::edit::quantize::note(start, end, grid);
            if let Some(l) = length.filter(|&l| start >= l) {
                (start, end) = (start - l, end - l);
            }
            (start, end, pitch, velocity)
        })
        .collect();
    placed.sort_by_key(|n| (n.0, n.2));
    let mut released = [0u64; 128];
    for (start, end, pitch, velocity) in placed {
        let p = usize::from(pitch & 0x7f);
        let start = start.max(released[p]);
        if length.is_some_and(|l| start >= l) {
            continue;
        }
        let end = end.max(start + grid).min(length.unwrap_or(u64::MAX));
        released[p] = end;
        out.push(note(start, pitch, velocity));
        out.push(note(end, pitch, 0));
    }
    // CC1 leads its tick; a Note Off and the next Note On of its pitch on one tick keep that order.
    out.sort_by_key(|e| (e.tick, e.status != 0xb0, e.status == 0x90));
    out
}

#[cfg(test)]
#[path = "quantize_tests.rs"]
mod tests;
