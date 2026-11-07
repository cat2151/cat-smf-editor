//! Persistent editing rows are separate from the complete catalog note list.
use super::*;
pub const DEFAULT: [Option<u8>; 4] = [Some(49), Some(42), Some(38), Some(36)];

/// Pitches recorded only into a Capture body add no row.
pub fn rows(track: &Track, kit: &Kit) -> Vec<Option<u8>> {
    let mut rows = kit.rows.clone().unwrap_or_else(|| DEFAULT.to_vec());
    let captures = captures(track);
    let mut extra: Vec<_> = track
        .events
        .iter()
        .filter(|e| note_on(e) && !captures.iter().any(|r| r.contains(&e.tick)))
        .map(|e| e.data[0])
        .filter(|n| !rows.contains(&Some(*n)))
        .collect();
    extra.sort_unstable_by(|a, b| b.cmp(a));
    extra.dedup();
    extra.into_iter().map(Some).chain(rows.drain(..)).collect()
}

fn captures(track: &Track) -> Vec<std::ops::Range<u64>> {
    use crate::score::pattern::{self, Kind, Record};
    track
        .events
        .iter()
        .filter_map(|e| match pattern::record(e) {
            Ok(Some(Record::Pattern(r))) if r.kind == Kind::Capture => {
                let length = pattern::body::length(&r.meters).ok()?;
                Some(e.tick..e.tick.checked_add(length)?)
            }
            _ => None,
        })
        .collect()
}

/// Remap the whole container, including every stored pattern body. No tick/velocity edits.
pub fn assign(track: &mut Track, kit: &mut Kit, row: usize, note: u8) -> Result<(), String> {
    if note > 127 {
        return Err("Invalid MIDI note".into());
    }
    let mut rows = rows(track, kit);
    let old = *rows.get(row).ok_or("Missing kit row")?;
    if rows
        .iter()
        .enumerate()
        .any(|(i, n)| i != row && *n == Some(note))
    {
        return Err("Note is already assigned to another row".into());
    }
    if let Some(old) = old {
        if let Some(swing) = kit.swing.remove(&old) {
            kit.swing.insert(note, swing);
        }
        if let Some(index) = kit.muted.iter().position(|&n| n == old) {
            kit.muted[index] = note;
            kit.muted.sort_unstable();
            kit.muted.dedup();
        }
        for e in &mut track.events {
            if (crate::score::note_view::note_on(e) || crate::score::note_view::note_off(e))
                && e.data[0] == old
            {
                e.data[0] = note;
            }
        }
        let velocity = kit.velocity(old);
        if !kit.notes.iter().any(|n| n.note == note) {
            kit.notes.push(Note {
                note,
                name: String::new(),
                velocity,
            });
            kit.notes.sort_by_key(|n| n.note);
        } else if let Some(n) = kit.notes.iter_mut().find(|n| n.note == note) {
            n.velocity = velocity;
        }
    }
    rows[row] = Some(note);
    kit.rows = Some(rows);
    kit.validate()
}
