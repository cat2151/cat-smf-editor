//! SMF's implicit 4/4 and C major, written explicitly at tick 0 of the conductor track.
use crate::{
    edit::{insert, refresh_deltas},
    model::{Event, Music},
    score::{key_signature::KeySignature, musical_time::valid_time_signature},
};

pub const FOUR_FOUR: [u8; 4] = [4, 2, 24, 8];
pub const C_MAJOR: [u8; 2] = [0, 0];

/// Adds each of 4/4 and C major only when no track has a readable one at tick 0.
pub fn initialize(music: &mut Music) -> Result<(), String> {
    if music.format != 1 || music.tracks.is_empty() {
        return Err("Initial meta events require format 1 track 0".into());
    }
    let at_zero = || {
        music
            .tracks
            .iter()
            .flat_map(|t| &t.events)
            .filter(|e| e.tick == 0)
    };
    let meter =
        at_zero().any(|e| e.status == 0xff && e.meta_type == 0x58 && valid_time_signature(&e.data));
    let key = at_zero().any(|e| KeySignature::from_event(e).is_some());
    let missing: Vec<_> = [(meter, 0x58, &FOUR_FOUR[..]), (key, 0x59, &C_MAJOR[..])]
        .into_iter()
        .filter(|(present, ..)| !present)
        .map(|(_, meta_type, data)| Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type,
            data: data.to_vec(),
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    for event in missing {
        insert(&mut music.tracks[0], event);
    }
    refresh_deltas(&mut music.tracks[0])
}

#[cfg(test)]
mod tests;
