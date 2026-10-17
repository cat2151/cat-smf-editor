use super::{read, write_start, Pattern, Region};
use crate::{
    edit::{refresh_deltas, tempo_curve::Curve},
    model::{Event, Music},
    score::note_view::{note_off, note_on},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod schedule;

pub(super) struct Plan {
    region: Region,
    pattern: Pattern,
    onsets: Vec<u64>,
    start_seconds: f64,
}

/// Removes only fully paired notes owned by each region. Crossing or ambiguous
/// same-pitch voices are errors, rather than deleting accompaniment by accident.
fn extract(music: &mut Music, regions: Vec<Region>) -> Result<Vec<Plan>, String> {
    for (i, a) in regions.iter().enumerate() {
        if regions[..i].iter().any(|b| {
            a.track == b.track
                && a.metadata.channel == b.metadata.channel
                && a.start < b.end
                && b.start < a.end
                && a.metadata
                    .notes
                    .iter()
                    .any(|n| b.metadata.notes.contains(n))
        }) {
            return Err("Trill regions overlap on the same channel and pitch".into());
        }
    }
    let mut plans = Vec::new();
    for (ti, track) in music.tracks.iter_mut().enumerate() {
        let mut pairs = vec![None; track.events.len()];
        let mut matched_offs = BTreeSet::new();
        let mut pending: BTreeMap<(u8, u8), VecDeque<usize>> = BTreeMap::new();
        for (i, e) in track.events.iter().enumerate() {
            if note_on(e) {
                pending
                    .entry((e.status & 15, e.data[0]))
                    .or_default()
                    .push_back(i);
            } else if note_off(e)
                && let Some(&pitch) = e.data.first()
                && let Some(on) = pending
                    .get_mut(&(e.status & 15, pitch))
                    .and_then(VecDeque::pop_front)
            {
                pairs[on] = Some(i);
                matched_offs.insert(i);
            }
        }
        let mut removed = BTreeSet::new();
        for region in regions.iter().filter(|r| r.track == ti) {
            if track.events.iter().enumerate().any(|(i, e)| {
                note_off(e)
                    && e.status & 15 == region.metadata.channel - 1
                    && e.data
                        .first()
                        .is_some_and(|n| region.metadata.notes.contains(n))
                    && region.start < e.tick
                    && e.tick <= region.end
                    && !matched_offs.contains(&i)
            }) {
                return Err("An unmatched Note Off lies inside a trill region".into());
            }
            let mut first = None;
            let mut velocities = [0; 2];
            let mut releases = [0; 2];
            let mut last_end = region.start;
            for (i, e) in track.events.iter().enumerate().filter(|(_, e)| {
                note_on(e)
                    && e.status & 15 == region.metadata.channel - 1
                    && region.metadata.notes.contains(&e.data[0])
            }) {
                let end = pairs[i].map(|off| track.events[off].tick);
                if e.tick < region.start && end.is_none_or(|end| end > region.start) {
                    return Err("A note crosses into a trill region".into());
                }
                if !(region.start..region.end).contains(&e.tick) {
                    continue;
                }
                let off = pairs[i].ok_or("A trill note has no Note Off")?;
                let end = end.unwrap();
                if end <= e.tick || end > region.end || e.tick < last_end {
                    return Err("Trill notes overlap or cross their region boundary".into());
                }
                last_end = end;
                first.get_or_insert(e.data[0]);
                let slot = usize::from(e.data[0] == region.metadata.notes[1]);
                if velocities[slot] == 0 {
                    velocities[slot] = e.data[1];
                    releases[slot] = track.events[off].data.get(1).copied().unwrap_or(0);
                }
                removed.extend([i, off]);
            }
            // Initial material supplies phase and velocity. Once captured, metadata
            // remains the source when an edit leaves fewer than two generated notes.
            let pattern = match &region.metadata.pattern {
                Some(pattern) => pattern.clone(),
                None => {
                    if velocities.contains(&0) {
                        return Err("A new trill region needs both source pitches".into());
                    }
                    Pattern {
                        first_note: first.unwrap(),
                        velocities,
                        release_velocities: releases,
                    }
                }
            };
            plans.push(Plan {
                region: region.clone(),
                pattern,
                onsets: Vec::new(),
                start_seconds: 0.0,
            });
        }
        let mut i = 0;
        track.events.retain(|_| {
            let keep = !removed.contains(&i);
            i += 1;
            keep
        });
    }
    Ok(plans)
}

pub(in crate::edit) fn rebuild(music: &mut Music, curve: Option<&Curve>) -> Result<(), String> {
    let regions = read(music)?;
    if regions.is_empty() {
        return Ok(());
    }
    let mut plans = extract(music, regions)?;
    schedule::run(music, curve, &mut plans)?;
    let mut generated: Vec<Vec<Event>> = vec![Vec::new(); music.tracks.len()];
    for plan in plans {
        let mut region = plan.region;
        region.metadata.pattern = Some(plan.pattern.clone());
        write_start(music, &region)?;
        let first_slot = usize::from(plan.pattern.first_note == region.metadata.notes[1]);
        let notes = &mut generated[region.track];
        for (i, &start) in plan.onsets.iter().enumerate() {
            let end = plan.onsets.get(i + 1).copied().unwrap_or(region.end);
            let slot = (first_slot + i) % 2;
            let pitch = region.metadata.notes[slot];
            for (tick, status, velocity) in [
                (start, 0x90, plan.pattern.velocities[slot]),
                (end, 0x80, plan.pattern.release_velocities[slot]),
            ] {
                notes.push(Event {
                    tick,
                    delta: 0,
                    status: status | (region.metadata.channel - 1),
                    meta_type: 0,
                    data: vec![pitch, velocity],
                });
            }
        }
    }
    for (track, mut generated) in music.tracks.iter_mut().zip(generated) {
        if generated.is_empty() {
            continue;
        }
        generated.append(&mut track.events);
        // Off before On includes the boundary with following non-trill notes and
        // adjacent managed regions. End of Track must remain last at its tick.
        generated.sort_by_key(|e| {
            (
                e.tick,
                if note_off(e) {
                    0
                } else if e.status == 0xff && e.meta_type == 0x2f {
                    3
                } else if note_on(e) {
                    2
                } else {
                    1
                },
            )
        });
        track.events = generated;
        refresh_deltas(track)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
