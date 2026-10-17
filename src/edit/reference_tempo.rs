//! Independent tempo guides, stored as English Text meta events at their own ticks.
use crate::{
    edit::{insert, refresh_deltas, tempo_curve::bpm_range},
    model::{Event, Music},
};
use std::collections::BTreeMap;

pub const PREFIX: &[u8] = b"Reference Tempo:";

pub fn owned(event: &Event) -> bool {
    event.status == 0xff && event.meta_type == 1 && event.data.starts_with(PREFIX)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub tick: u64,
    pub bpm: f64,
    /// Dense rank, fastest first; captured once and never changed by dragging.
    pub rank: usize,
}

/// Dense ranks: equal BPMs share a rank, with no gaps between ranks.
pub fn ranks(points: &[Reference]) -> Vec<usize> {
    let mut bpms: Vec<_> = points.iter().map(|p| p.bpm).collect();
    bpms.sort_by(|a, b| b.total_cmp(a));
    bpms.dedup();
    points
        .iter()
        .map(|p| bpms.partition_point(|b| *b > p.bpm) + 1)
        .collect()
}

pub fn read(music: &Music) -> Result<Vec<Reference>, String> {
    let mut points = BTreeMap::new();
    for (track, t) in music.tracks.iter().enumerate() {
        for event in t.events.iter().filter(|e| owned(e)) {
            if music.format != 1 || track != 0 {
                return Err("Reference Tempo requires format 1 track 0".into());
            }
            let (value, rank) = std::str::from_utf8(&event.data[PREFIX.len()..])
                .ok()
                .and_then(|s| s.trim().split_once(" BPM; rank="))
                .ok_or("Invalid Reference Tempo metadata")?;
            let bpm = value
                .parse::<f64>()
                .ok()
                .filter(|b| bpm_range().contains(b))
                .ok_or("Invalid Reference Tempo BPM")?;
            let rank = rank
                .parse::<usize>()
                .ok()
                .filter(|r| *r > 0)
                .ok_or("Invalid Reference Tempo rank")?;
            if points
                .insert(
                    event.tick,
                    Reference {
                        tick: event.tick,
                        bpm,
                        rank,
                    },
                )
                .is_some()
            {
                return Err("Duplicate Reference Tempo tick".into());
            }
        }
    }
    Ok(points.into_values().collect())
}

/// Captures existing tempos once; an implicit initial tempo is SMF's default 120 BPM.
pub fn initialize(music: &mut Music) -> Result<(), String> {
    if !read(music)?.is_empty() {
        return Ok(());
    }
    if music.format != 1 || music.tracks.is_empty() {
        return Err("Reference Tempo requires format 1 track 0".into());
    }
    let mut points = BTreeMap::from([(0, 120.0)]);
    for e in &music.tracks[0].events {
        if e.status == 0xff && e.meta_type == 0x51 {
            let [a, b, c] = e.data.as_slice() else {
                return Err("Invalid tempo event".into());
            };
            let micros = u32::from_be_bytes([0, *a, *b, *c]);
            if micros == 0 {
                return Err("Invalid zero tempo".into());
            }
            points.insert(e.tick, 60_000_000.0 / f64::from(micros));
        }
    }
    let points: Vec<_> = points
        .into_iter()
        .map(|(tick, bpm)| Reference { tick, bpm, rank: 0 })
        .collect();
    for (mut point, rank) in points.iter().copied().zip(ranks(&points)) {
        point.rank = rank;
        insert(&mut music.tracks[0], event(point));
    }
    refresh_deltas(&mut music.tracks[0])
}

fn event(point: Reference) -> Event {
    Event {
        tick: point.tick,
        delta: 0,
        status: 0xff,
        meta_type: 1,
        data: format!("Reference Tempo: {} BPM; rank={}", point.bpm, point.rank).into_bytes(),
    }
}

/// Changes only the BPM of an existing guide. Its tick and every other event stay.
pub fn set(music: &mut Music, tick: u64, bpm: f64) -> Result<(), String> {
    let points = read(music)?;
    if !bpm_range().contains(&bpm) {
        return Err("Invalid Reference Tempo BPM".into());
    }
    let original = points
        .iter()
        .find(|p| p.tick == tick)
        .ok_or("No Reference Tempo at this tick")?;
    let target = music.tracks[0]
        .events
        .iter_mut()
        .find(|e| e.tick == tick && owned(e))
        .unwrap();
    target.data = event(Reference { bpm, ..*original }).data;
    Ok(())
}

#[cfg(test)]
mod tests;
