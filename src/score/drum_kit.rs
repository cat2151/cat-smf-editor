//! Actual MIDI pitches, independent of displayed row order or catalog availability.
pub use crate::drum_kit::{Kit, Note};
pub mod layout;
use crate::{
    edit,
    model::{Event, Music, Track},
    score::{drums::Measure, musical_time::MusicalTime, note_view::note_on},
};

pub fn read(track: &Track) -> Result<Option<Kit>, String> {
    Ok(crate::instrument_layers::parse(track)?.kit)
}
pub fn rows(track: &Track, kit: &Kit) -> Vec<Note> {
    let mut notes: std::collections::BTreeMap<_, _> =
        kit.notes.iter().map(|n| (n.note, n.clone())).collect();
    for e in track.events.iter().filter(|e| note_on(e)) {
        notes.entry(e.data[0]).or_insert_with(|| Note {
            note: e.data[0],
            name: String::new(),
            velocity: 100,
        });
    }
    notes.into_values().collect()
}
pub fn hits<'a>(
    track: &'a Track,
    measure: &Measure,
    note: u8,
    step: usize,
) -> Vec<(usize, &'a Event)> {
    let Some(&start) = measure.steps.get(step) else {
        return vec![];
    };
    let end = measure.steps.get(step + 1).copied().unwrap_or(measure.end);
    track
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| note_on(e) && e.data[0] == note && (start..end).contains(&e.tick))
        .collect()
}
pub fn toggle(
    music: &mut Music,
    track: usize,
    note: u8,
    number: u64,
    step: usize,
) -> Result<(), String> {
    if note > 127 {
        return Err("Invalid MIDI note".into());
    }
    let kit = read(music.tracks.get(track).ok_or("Missing kit track")?)?.ok_or("Missing kit")?;
    toggle_step(music, track, note, number, step, kit.velocity(note))
}

/// Pattern projections intentionally omit container metadata; their caller owns the default.
pub fn toggle_step(
    music: &mut Music,
    track: usize,
    note: u8,
    number: u64,
    step: usize,
    velocity: u8,
) -> Result<(), String> {
    if note > 127 || !(1..=127).contains(&velocity) {
        return Err("Invalid kit note or velocity".into());
    }
    let time = MusicalTime::new(music);
    let measure = Measure::new(
        time.map(track).ok_or("Missing measure map")?,
        music.division,
        number,
    )?;
    let start = *measure.steps.get(step).ok_or("Invalid step")?;
    let end = measure.steps.get(step + 1).copied().unwrap_or(measure.end);
    let source = &mut music.tracks[track];
    let remove: Vec<_> = hits(source, &measure, note, step)
        .iter()
        .map(|(i, _)| *i)
        .collect();
    if remove.is_empty() {
        for (tick, status, velocity) in [(start, 0x99, velocity), (end, 0x89, 0)] {
            edit::insert(
                source,
                Event {
                    tick,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![note, velocity],
                },
            );
        }
    } else {
        edit::event_range::delete(source, &remove, true)?;
    }
    edit::refresh_deltas(source)?;
    crate::score::drums::rebuild(music)
}
pub fn velocity(
    music: &mut Music,
    track: usize,
    note: u8,
    number: u64,
    step: usize,
    value: u8,
) -> Result<(), String> {
    if !(1..=127).contains(&value) {
        return Err("Velocity must be 1..127".into());
    }
    let time = MusicalTime::new(music);
    let measure = Measure::new(
        time.map(track).ok_or("Missing measure map")?,
        music.division,
        number,
    )?;
    let source = music.tracks.get_mut(track).ok_or("Missing kit track")?;
    let indices: Vec<_> = hits(source, &measure, note, step)
        .iter()
        .map(|(i, _)| *i)
        .collect();
    if indices.is_empty() {
        return Err("Empty step".into());
    }
    for i in indices {
        source.events[i].data[1] = value;
    }
    Ok(())
}
#[cfg(test)]
mod tests;
