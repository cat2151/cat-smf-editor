//! Recognize rendered trills and retain their speed in seconds.
use crate::{
    edit::{insert, refresh_deltas},
    model::{Event, Music},
    score::{
        audition_timing::TempoMap,
        note_view::{note_on, NoteView},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const START: &[u8] = b"Trill Start: ";
const END: &[u8] = b"Trill End: ";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub version: u8,
    pub id: usize,
    /// MIDI channel, one-based.
    pub channel: u8,
    pub notes: [u8; 2],
    /// Time from one Note On to the next, rather than a complete two-note cycle.
    pub interval_seconds: f64,
    /// Optional independent first-note length; omission follows interval_seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_note_seconds: Option<f64>,
    /// Retained even when a slow interval leaves just one pitch in the region.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pattern {
    pub first_note: u8,
    pub velocities: [u8; 2],
    pub release_velocities: [u8; 2],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct End {
    version: u8,
    id: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub track: usize,
    pub start: u64,
    /// Exclusive end, at the last trill note's Note Off.
    pub end: u64,
    pub metadata: Start,
}

pub fn owned(e: &Event) -> bool {
    e.status == 0xff && e.meta_type == 1 && (e.data.starts_with(START) || e.data.starts_with(END))
}

pub fn read(music: &Music) -> Result<Vec<Region>, String> {
    let mut out = Vec::new();
    for (track, source) in music.tracks.iter().enumerate() {
        let mut pending = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for e in source.events.iter().filter(|e| owned(e)) {
            if music.format != 1 || music.division == 0 || music.division & 0x8000 != 0 {
                return Err("Trill metadata requires format 1 with PPQN timing".into());
            }
            if let Some(data) = e.data.strip_prefix(START) {
                let start: Start = serde_json::from_slice(data).map_err(|e| e.to_string())?;
                if start.version != 1
                    || start.id == 0
                    || !(1..=16).contains(&start.channel)
                    || start.notes[1] > 127
                    || start.notes[0] >= start.notes[1]
                    || start.notes[1] - start.notes[0] > 2
                    || !start.interval_seconds.is_finite()
                    || start.interval_seconds <= 0.0
                    || start
                        .first_note_seconds
                        .is_some_and(|s| !s.is_finite() || s <= 0.0)
                    || start.pattern.as_ref().is_some_and(|p| {
                        !start.notes.contains(&p.first_note)
                            || p.velocities.iter().any(|v| *v == 0 || *v > 127)
                            || p.release_velocities.iter().any(|v| *v > 127)
                    })
                    || !ids.insert(start.id)
                {
                    return Err("Invalid Trill Start metadata".into());
                }
                pending.insert(start.id, (e.tick, start));
            } else {
                let end: End =
                    serde_json::from_slice(&e.data[END.len()..]).map_err(|e| e.to_string())?;
                let (tick, start) = pending.remove(&end.id).ok_or("Unmatched Trill End")?;
                if end.version != 1 || e.tick <= tick {
                    return Err("Invalid Trill End metadata".into());
                }
                out.push(Region {
                    track,
                    start: tick,
                    end: e.tick,
                    metadata: start,
                });
            }
        }
        if !pending.is_empty() {
            return Err("Unmatched Trill Start".into());
        }
    }
    out.sort_by_key(|r| (r.track, r.start, r.metadata.id));
    Ok(out)
}

/// At least six alternating semitone/whole-tone notes, at equal intervals no longer
/// than a 64th note. Every note must end at the next onset, including the last.
/// Other pitches may accompany the trill on the same channel.
pub fn detect(music: &Music) -> Result<Vec<Region>, String> {
    if music.format != 1 || music.division == 0 || music.division & 0x8000 != 0 {
        return Err("Trill detection requires format 1 with PPQN timing".into());
    }
    let mut out = Vec::new();
    for (track, source) in music.tracks.iter().enumerate() {
        let view = NoteView::new(&source.events);
        let map = TempoMap::new(music, track);
        let mut candidates = Vec::new();
        for channel in 0..16 {
            for low in 0u8..127 {
                for high in (low + 1)..=low.saturating_add(2).min(127) {
                    let seq: Vec<_> = source
                        .events
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| {
                            note_on(e)
                                && e.status & 15 == channel
                                && e.data.first().is_some_and(|n| *n == low || *n == high)
                        })
                        .collect();
                    let mut i = 0;
                    while i + 1 < seq.len() {
                        let gap = seq[i + 1].1.tick.saturating_sub(seq[i].1.tick);
                        let valid = |a: usize, b: usize| {
                            gap > 0
                                && gap <= u64::from(music.division) / 16
                                && seq[b].1.tick.checked_sub(seq[a].1.tick) == Some(gap)
                                && seq[a].1.data[0] != seq[b].1.data[0]
                                && view.durations[seq[a].0] == Some(gap)
                                && view.durations[seq[b].0] == Some(gap)
                        };
                        if !valid(i, i + 1) {
                            i += 1;
                            continue;
                        }
                        let mut j = i + 1;
                        while j + 1 < seq.len() && valid(j, j + 1) {
                            j += 1;
                        }
                        if j - i + 1 >= 6 {
                            let start = seq[i].1.tick;
                            let end = seq[j]
                                .1
                                .tick
                                .checked_add(gap)
                                .ok_or("Trill end tick overflow")?;
                            let interval_seconds = map.duration(start, gap);
                            if !interval_seconds.is_finite() || interval_seconds <= 0.0 {
                                return Err("Invalid detected trill interval".into());
                            }
                            candidates.push(Region {
                                track,
                                start,
                                end,
                                metadata: Start {
                                    version: 1,
                                    id: 0,
                                    channel: channel + 1,
                                    notes: [low, high],
                                    interval_seconds,
                                    first_note_seconds: None,
                                    pattern: None,
                                },
                            });
                        }
                        i = j;
                    }
                }
            }
        }
        candidates.sort_by_key(|r| (r.start, r.metadata.channel, r.metadata.notes));
        // Ambiguous runs sharing a pitch and channel cannot safely own the same notes.
        for (i, candidate) in candidates.iter().enumerate() {
            let ambiguous = candidates.iter().enumerate().any(|(j, other)| {
                i != j
                    && candidate.metadata.channel == other.metadata.channel
                    && candidate.start < other.end
                    && other.start < candidate.end
                    && candidate
                        .metadata
                        .notes
                        .iter()
                        .any(|n| other.metadata.notes.contains(n))
            });
            if !ambiguous {
                let mut region = candidate.clone();
                region.metadata.id = out.iter().filter(|r: &&Region| r.track == track).count() + 1;
                out.push(region);
            }
        }
    }
    Ok(out)
}

pub(super) fn write_start(music: &mut Music, region: &Region) -> Result<(), String> {
    let mut data = START.to_vec();
    data.extend(serde_json::to_vec(&region.metadata).map_err(|e| e.to_string())?);
    let event = music.tracks[region.track]
        .events
        .iter_mut()
        .find(|e| {
            owned(e)
                && e.tick == region.start
                && e.data.starts_with(START)
                && serde_json::from_slice::<Start>(&e.data[START.len()..])
                    .is_ok_and(|m| m.id == region.metadata.id)
        })
        .ok_or("Missing Trill Start")?;
    event.data = data;
    Ok(())
}

impl Start {
    pub fn onset_seconds(&self, index: usize) -> f64 {
        if index == 0 {
            0.0
        } else {
            self.first_note_seconds.unwrap_or(self.interval_seconds)
                + (index - 1) as f64 * self.interval_seconds
        }
    }
}

/// Both Start and End text open the same managed region.
pub fn for_event(music: &Music, track: usize, index: usize) -> Result<Option<Region>, String> {
    let e = music
        .tracks
        .get(track)
        .and_then(|t| t.events.get(index))
        .ok_or("Unknown event")?;
    if !owned(e) {
        return Ok(None);
    }
    let id = if let Some(data) = e.data.strip_prefix(START) {
        serde_json::from_slice::<Start>(data)
            .map_err(|e| e.to_string())?
            .id
    } else {
        serde_json::from_slice::<End>(&e.data[END.len()..])
            .map_err(|e| e.to_string())?
            .id
    };
    read(music)?
        .into_iter()
        .find(|r| r.track == track && r.metadata.id == id)
        .map(Some)
        .ok_or_else(|| "Unmatched trill metadata".into())
}

pub fn label(event: &Event) -> Option<&'static str> {
    owned(event).then(|| {
        if event.data.starts_with(START) {
            "Trill Start"
        } else {
            "Trill End"
        }
    })
}

/// Selects the starting pitch while retaining timing and pitch-specific velocities.
pub fn set_first_note(
    music: &mut Music,
    track: usize,
    id: usize,
    upper: bool,
) -> Result<(), String> {
    let find = |music: &Music| {
        read(music)?
            .into_iter()
            .find(|r| r.track == track && r.metadata.id == id)
            .ok_or_else(|| "Unknown trill".to_owned())
    };
    let mut region = find(music)?;
    if region.metadata.pattern.is_none() {
        super::regenerate(music)?;
        region = find(music)?;
    }
    region
        .metadata
        .pattern
        .as_mut()
        .ok_or("Missing trill pattern")?
        .first_note = region.metadata.notes[usize::from(upper)];
    write_start(music, &region)
}

pub fn set_timing(
    music: &mut Music,
    track: usize,
    id: usize,
    interval: f64,
    first: f64,
) -> Result<(), String> {
    if [interval, first]
        .iter()
        .any(|s| !s.is_finite() || *s <= 0.0)
    {
        return Err("Trill durations must be finite and positive".into());
    }
    let mut region = read(music)?
        .into_iter()
        .find(|r| r.track == track && r.metadata.id == id)
        .ok_or("Unknown trill")?;
    region.metadata.interval_seconds = interval;
    region.metadata.first_note_seconds = Some(first);
    write_start(music, &region)
}

/// Updates the managed speed; the shared edit pipeline rebuilds notes afterwards.
pub fn set_interval(
    music: &mut Music,
    track: usize,
    id: usize,
    seconds: f64,
) -> Result<(), String> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("Trill interval must be finite and positive".into());
    }
    let mut region = read(music)?
        .into_iter()
        .find(|r| r.track == track && r.metadata.id == id)
        .ok_or("Unknown trill")?;
    region.metadata.interval_seconds = seconds;
    write_start(music, &region)
}

/// Managed notes must be edited through their interval rather than moved out of
/// the owning region, leaving an orphan alongside regenerated notes.
pub fn ensure_note_editable(music: &Music, track: usize, index: usize) -> Result<(), String> {
    let event = music
        .tracks
        .get(track)
        .and_then(|t| t.events.get(index))
        .ok_or("Unknown note")?;
    if note_on(event)
        && read(music)?.iter().any(|r| {
            r.track == track
                && r.metadata.channel - 1 == event.status & 15
                && r.start <= event.tick
                && event.tick < r.end
                && r.metadata.notes.contains(&event.data[0])
        })
    {
        return Err("Managed trill notes use the Trill interval".into());
    }
    Ok(())
}

mod rebuild;
pub(super) use rebuild::rebuild;

/// Once any managed trill exists, reopening never re-detects or overwrites its speed.
pub fn initialize(music: &mut Music) -> Result<(), String> {
    if !read(music)?.is_empty() {
        return Ok(());
    }
    let regions = detect(music)?;
    for region in regions {
        let mut start = START.to_vec();
        start.extend(serde_json::to_vec(&region.metadata).map_err(|e| e.to_string())?);
        let mut end = END.to_vec();
        end.extend(
            serde_json::to_vec(&End {
                version: 1,
                id: region.metadata.id,
            })
            .map_err(|e| e.to_string())?,
        );
        let track = &mut music.tracks[region.track];
        for (tick, data) in [(region.start, start), (region.end, end)] {
            insert(
                track,
                Event {
                    tick,
                    delta: 0,
                    status: 0xff,
                    meta_type: 1,
                    data,
                },
            );
        }
        refresh_deltas(track)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
