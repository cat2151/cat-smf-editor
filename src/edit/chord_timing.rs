//! A chord is the notes whose Note On shares one tick; it moves, stretches and snaps as one.
use crate::{
    edit::{note_times::retime, quantize, relocate::pairs},
    model::Track,
    score::note_view::note_on,
};

/// What one key press does to a chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Note On and Note Off move together by this many ticks, so lengths stay.
    Move(i64),
    /// Note Off moves by this many ticks.
    Length(i64),
    /// Start and end snap to this grid.
    Quantize(u64),
}

/// Ticks where a note with a Note Off starts, ascending and once each.
pub fn onsets(track: &Track) -> Vec<u64> {
    let offs = pairs(track.events.iter());
    let mut ticks: Vec<u64> = track
        .events
        .iter()
        .enumerate()
        .filter(|&(i, e)| note_on(e) && offs[i].is_some())
        .map(|(_, e)| e.tick)
        .collect();
    ticks.dedup();
    ticks
}

/// Note On indices of the notes starting at `tick` that have a Note Off.
pub fn notes(track: &Track, tick: u64) -> Vec<usize> {
    let offs = pairs(track.events.iter());
    track
        .events
        .iter()
        .enumerate()
        .filter(|&(i, e)| e.tick == tick && note_on(e) && offs[i].is_some())
        .map(|(i, _)| i)
        .collect()
}

/// Changes the chord starting at `tick` and returns where it starts afterwards.
/// On error the track is unchanged.
pub fn apply(track: &mut Track, tick: u64, change: Change) -> Result<u64, String> {
    let offs = pairs(track.events.iter());
    let chord = notes(track, tick);
    if chord.is_empty() {
        return Err("このtickから始まるnoteがありません".into());
    }
    let shifted = |t: u64, by: i64| {
        t.checked_add_signed(by)
            .ok_or_else(|| "曲の先頭より前へは動かせません".to_string())
    };
    let mut moves = Vec::with_capacity(chord.len());
    for &on in &chord {
        let start = track.events[on].tick;
        let end = track.events[offs[on].unwrap()].tick;
        moves.push(match change {
            Change::Move(by) => (on, shifted(start, by)?, shifted(end, by)?),
            Change::Length(by) => (on, start, shifted(end, by)?),
            Change::Quantize(grid) => {
                let (start, end) = quantize::note(start, end, grid);
                (on, start, end)
            }
        });
    }
    retime(track, &moves)?;
    Ok(match change {
        Change::Move(by) => shifted(tick, by)?,
        Change::Length(_) => tick,
        Change::Quantize(grid) => quantize::snap(tick, grid),
    })
}

#[cfg(test)]
mod tests;
