//! A transient local/destination projection mapped atomically to one stable pattern ID.
use super::*;
use crate::{edit, model::Track, score::note_view::note_off};
type EditRanges = (Definition, (u64, u64), (u64, u64));
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub track: usize,
    pub measure: u128,
    pub pattern: u64,
    pub placement: Option<u64>,
    pub scene: Option<u64>,
    pub standalone: bool,
}
impl Context {
    pub fn rebind(&mut self, music: &Music) -> Result<(), String> {
        let c = Catalog::read(music)?;
        let p = c
            .patterns
            .get(&self.pattern)
            .ok_or("Pattern edit target no longer exists")?;
        if self.standalone {
            self.track = p.track;
            self.measure = 1;
        }
        Ok(())
    }
    fn ranges(&self, music: &Music) -> Result<EditRanges, String> {
        let c = Catalog::read(music)?;
        let p = c
            .patterns
            .get(&self.pattern)
            .ok_or("Pattern edit target no longer exists")?
            .clone();
        let source = p.range()?;
        if self.standalone {
            if self.track != p.track || self.scene.is_some() {
                return Err("Pattern edit target changed".into());
            }
            let length = p.length()?;
            return Ok((p, source, (0, length)));
        }
        let id = self.scene.ok_or("Pattern edit has no placement")?;
        let collection = crate::score::scene::read(music)?;
        let scene = collection
            .scenes
            .iter()
            .find(|s| s.id == id)
            .ok_or("Missing edit scene")?;
        if self.track != p.track
            || !scene.cells.iter().any(|cell| {
                u128::from(cell.measure) + 1 == self.measure
                    && cell.pattern == self.pattern
                    && self.placement.is_none_or(|id| id == cell.id)
            })
        {
            return Err("Scene edit placement changed".into());
        }
        let start = scene.range(self.measure)?.0;
        let end = start
            .checked_add(p.length()?)
            .ok_or("Edit range overflow")?;
        Ok((p, source, (start, end)))
    }
    pub fn edit_range(&self, music: &Music) -> Result<(u64, u64), String> {
        Ok(self.ranges(music)?.2)
    }
    pub fn project(&self, music: &Music) -> Result<Music, String> {
        let (p, source, dest) = self.ranges(music)?;
        if let Some(id) = self.scene {
            let c = Catalog::read(music)?;
            let collection = crate::score::scene::read(music)?;
            let scene = collection
                .scenes
                .iter()
                .find(|s| s.id == id)
                .ok_or("Missing edit scene")?;
            return crate::score::scene::projection::music(music, scene, &c);
        }
        let mut next = music.clone();
        if self.standalone {
            for t in &mut next.tracks {
                t.events
                    .retain(|e| !(e.status == 0xff && e.meta_type == 0x58));
            }
            next.tracks[self.track].events.clear();
            let meter = p.meter_music(music.division)?;
            for e in &meter.tracks[0].events {
                edit::insert(&mut next.tracks[0], e.clone());
            }
        }
        for mut e in body::extract(&music.tracks[p.track].events, source)? {
            e.tick = dest.0 + (e.tick - source.0);
            edit::insert(&mut next.tracks[self.track], e);
        }
        if self.standalone {
            edit::insert(
                &mut next.tracks[self.track],
                Event {
                    tick: dest.1,
                    delta: 0,
                    status: 0xff,
                    meta_type: 0x2f,
                    data: vec![],
                },
            );
        }
        for t in &mut next.tracks {
            edit::refresh_deltas(t)?;
        }
        Ok(next)
    }
    pub fn map(&self, base: &Music, view: &Music, edited: &Music) -> Result<Music, String> {
        let (p, source, dest) = self.ranges(base)?;
        let changed = edited.tracks.get(self.track).ok_or("Missing edit track")?;
        let events = body::extract(&changed.events, dest)?;
        let masked = |m: &Music| -> Result<Vec<u8>, String> {
            let mut copy = m.clone();
            remove(&mut copy.tracks[self.track], dest);
            copy.tracks[self.track]
                .events
                .retain(|e| !(e.status == 0xff && e.meta_type == 0x2f));
            crate::smf_io::encode(&copy)
        };
        if masked(view)? != masked(edited)? {
            return Err("Edit exceeds the selected pattern body".into());
        }
        let mut next = base.clone();
        let track = &mut next.tracks[p.track];
        remove(track, source);
        for mut e in events {
            e.tick = source.0 + (e.tick - dest.0);
            edit::insert(track, e);
        }
        edit::refresh_deltas(track)?;
        Catalog::read(&next)?;
        Ok(next)
    }
}
fn remove(track: &mut Track, (start, end): (u64, u64)) {
    track.events.retain(|e| {
        !(body::expression(e) && ((start..end).contains(&e.tick) || e.tick == end && note_off(e)))
    });
}
