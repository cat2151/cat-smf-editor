//! Quantize: a note starts and ends on the nearest grid line, lasting at least one step.
use crate::{
    edit::{note_times::retime, relocate::pairs},
    model::Track,
    score::note_view::note_on,
};

/// Ticks of a sixteenth note at `division` ticks per quarter.
pub fn sixteenth(division: u16) -> Result<u64, String> {
    if division & 0x8000 != 0 || division < 4 {
        return Err("quantizeには4以上のticks per quarterが必要です".into());
    }
    Ok(u64::from(division) / 4)
}

/// The nearest multiple of `grid`; halfway goes later.
pub fn snap(tick: u64, grid: u64) -> u64 {
    (tick + grid / 2) / grid * grid
}

/// The start and end of a note from `start` to `end` on the grid.
pub fn note(start: u64, end: u64, grid: u64) -> (u64, u64) {
    let start = snap(start, grid);
    (start, snap(end, grid).max(start + grid))
}

/// Quantizes the notes whose Note On is among `selected`; other events are ignored.
pub fn notes(track: &mut Track, selected: &[usize], grid: u64) -> Result<(), String> {
    let offs = pairs(track.events.iter());
    let mut moves = vec![];
    for &on in selected {
        let Some(e) = track.events.get(on).filter(|e| note_on(e)) else {
            continue;
        };
        let Some(off) = offs[on] else {
            continue;
        };
        let (start, end) = note(e.tick, track.events[off].tick, grid);
        moves.push((on, start, end));
    }
    retime(track, &moves)
}

#[cfg(test)]
mod tests;
