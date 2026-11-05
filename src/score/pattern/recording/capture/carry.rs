//! Preserve captures when undo restores an older version of the song.
use super::*;
use crate::model::Track;
use crate::score::pattern::{event, PatternRecord, Record, VERSION};

/// Copies every Capture pattern of `from` into `into`, replacing the pattern of the same id.
/// A container `into` lacks comes along with its track events outside patterns.
pub fn carry(from: &Music, into: &mut Music) -> Result<(), String> {
    let source = Catalog::read(from)?;
    let captures: Vec<_> = source
        .patterns
        .values()
        .filter(|p| p.kind == Kind::Capture)
        .collect();
    if captures.is_empty() {
        return Ok(());
    }
    let mut next = into.clone();
    for p in captures {
        let c = Catalog::read(&next)?;
        if let Some(old) = c.patterns.get(&p.id) {
            operations::remove(&mut next.tracks[old.track], old)?;
        }
        let track = match c.tracks.get(&p.container) {
            Some(&track) => track,
            None => {
                next.tracks.push(container(from, &source, p.track)?);
                if next.format == 0 {
                    next.format = 1;
                }
                next.tracks.len() - 1
            }
        };
        let tick = operations::free_tick(&Catalog::read(&next)?, track)?;
        let mut events = body::extract(&from.tracks[p.track].events, p.range()?)?;
        for e in &mut events {
            e.tick = tick
                .checked_add(e.tick - p.tick)
                .ok_or(message::STORAGE_OVERFLOW)?;
        }
        events.push(event(
            tick,
            &Record::Pattern(PatternRecord {
                version: VERSION,
                id: p.id,
                kind: p.kind,
                name: p.name.clone(),
                meters: p.meters.clone(),
            }),
        ));
        for e in events {
            edit::insert(&mut next.tracks[track], e);
        }
        edit::refresh_deltas(&mut next.tracks[track])?;
    }
    Catalog::read(&next)?;
    *into = next;
    Ok(())
}

/// The events of container `track` outside its patterns.
fn container(from: &Music, c: &Catalog, track: usize) -> Result<Track, String> {
    let mut t = from.tracks[track].clone();
    for p in c.patterns.values().filter(|p| p.track == track) {
        operations::remove(&mut t, p)?;
    }
    Ok(t)
}
