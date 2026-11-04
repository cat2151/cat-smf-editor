//! Scene coordinates are transient; original song notes and meter events stay saved.
use super::*;
use crate::message;
use crate::score::note_view::{note_off, note_on};
pub fn music(base: &Music, scene: &Scene, c: &Catalog) -> Result<Music, String> {
    let mut next = base.clone();
    next.format = 1;
    for track in &mut next.tracks {
        track.events.retain(|e| {
            !note_on(e)
                && !note_off(e)
                && !(e.status == 0xff && (e.meta_type == 0x58 || e.meta_type == 0x2f))
                && !same_measure::owned(e)
        });
    }
    let meter = scene.meter_music(base.division)?;
    for e in &meter.tracks[0].events {
        crate::edit::insert(&mut next.tracks[0], e.clone());
    }
    for cell in &scene.cells {
        let p = c
            .patterns
            .get(&cell.pattern)
            .ok_or(message::MISSING_SCENE_PATTERN)?;
        let track = p.track;
        let source = p.range()?;
        let dest = scene.range(u128::from(cell.measure) + 1)?;
        let notes = if crate::score::drums::read(&base.tracks[p.track])?.is_some() {
            crate::score::drums::pattern_body(base, p)?
                .into_iter()
                .filter(|e| note_on(e) || note_off(e))
                .collect()
        } else {
            same_measure::notes(&base.tracks[p.track].events, source)?
        };
        for mut e in notes {
            e.tick = dest.0 + (e.tick - source.0);
            crate::edit::insert(&mut next.tracks[track], e);
        }
    }
    let end = scene.range(scene.meters.len() as u128)?.1;
    for track in &mut next.tracks {
        crate::edit::insert(
            track,
            Event {
                tick: end.max(track.events.last().map_or(0, |e| e.tick)),
                delta: 0,
                status: 0xff,
                meta_type: 0x2f,
                data: vec![],
            },
        );
        crate::edit::refresh_deltas(track)?;
    }
    Ok(next)
}
