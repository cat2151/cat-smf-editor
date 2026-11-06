//! Pattern storage ticks are not musical time: releases stay inside each body.
use super::*;
use crate::{
    model::Music,
    score::pattern::{self, Definition},
};

pub(crate) fn pattern_body(music: &Music, p: &Definition) -> Result<Vec<Event>, String> {
    let mut events = pattern::body::extract(&music.tracks[p.track].events, p.range()?)?;
    let kit = crate::score::drum_kit::read(&music.tracks[p.track])?.is_some();
    if read(&music.tracks[p.track])?.is_some() || kit {
        let mut end = p.tick;
        let ends: Vec<_> = p
            .meters
            .iter()
            .map(|m| {
                end += m.ticks;
                end
            })
            .collect();
        extend(&mut events, p.range()?.1, kit, &ends)?;
    }
    Ok(events)
}

fn extend(events: &mut [Event], end: u64, kit: bool, ends: &[u64]) -> Result<(), String> {
    let pairs = edit::relocate::pairs(events.iter());
    let mut releases = Vec::new();
    for (index, on) in events.iter().enumerate() {
        // The step grid writes channel 10; other channels keep their performed releases.
        if !note_on(on) || on.status != 0x99 || (!kit && !NOTES.contains(&on.data[0])) {
            continue;
        }
        let next = events[index + 1..].iter().find(|e| {
            note_on(e) && e.status == on.status && e.data[0] == on.data[0] && e.tick > on.tick
        });
        let release = if kit {
            let end = ends.iter().copied().find(|&t| t > on.tick).unwrap_or(end);
            next.map_or(end, |e| e.tick.min(end))
        } else {
            next.map_or(end, |e| (e.tick - 1).max(on.tick + 1).min(e.tick))
        };
        releases.push((
            pairs[index].ok_or("drumのnoteにNote Offがありません")?,
            release,
        ));
    }
    for (off, tick) in releases {
        events[off].tick = tick;
    }
    events.sort_by_key(|e| {
        (
            e.tick,
            !(e.status & 15 == 9 && crate::score::note_view::note_off(e)),
        )
    });
    Ok(())
}

pub(crate) fn rebuild(music: &mut Music) -> Result<(), String> {
    if !pattern::has_containers(music) {
        return Ok(());
    }
    let catalog = pattern::Catalog::read(music)?;
    for p in catalog.patterns.values() {
        if read(&music.tracks[p.track])?.is_none()
            && crate::score::drum_kit::read(&music.tracks[p.track])?.is_none()
        {
            continue;
        }
        let events = pattern_body(music, p)?;
        let (start, end) = p.range()?;
        let track = &mut music.tracks[p.track];
        track.events.retain(|e| {
            !(pattern::body::expression(e)
                && ((start..end).contains(&e.tick)
                    || e.tick == end && crate::score::note_view::note_off(e)))
        });
        for event in events {
            edit::insert(track, event);
        }
        edit::refresh_deltas(track)?;
    }
    Ok(())
}
