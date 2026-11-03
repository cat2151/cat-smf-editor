//! A transient local/destination projection mapped atomically to one stable pattern ID.
use super::*;
use crate::message;
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
            .ok_or(message::PATTERN_EDIT_TARGET_NO_LONGER_EXISTS)?;
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
            .ok_or(message::PATTERN_EDIT_TARGET_NO_LONGER_EXISTS)?
            .clone();
        let source = p.range()?;
        if self.standalone {
            if self.track != p.track || self.scene.is_some() {
                return Err("patternの編集対象が変わりました".into());
            }
            let length = p.length()?;
            return Ok((p, source, (0, length)));
        }
        let id = self.scene.ok_or("編集中のpatternに配置がありません")?;
        let collection = crate::score::scene::read(music)?;
        let scene = collection
            .scenes
            .iter()
            .find(|s| s.id == id)
            .ok_or(message::MISSING_EDIT_SCENE)?;
        if self.track != p.track
            || !scene.cells.iter().any(|cell| {
                u128::from(cell.measure) + 1 == self.measure
                    && cell.pattern == self.pattern
                    && self.placement.is_none_or(|id| id == cell.id)
            })
        {
            return Err("編集中のsceneの配置が変わりました".into());
        }
        let start = scene.range(self.measure)?.0;
        let end = start
            .checked_add(p.length()?)
            .ok_or("編集範囲が上限を超えました")?;
        Ok((p, source, (start, end)))
    }
    /// Crops the pattern to `selected` events of the edited track of `view` (the projection of
    /// `music`). A note stays whole when its Note On is selected and goes otherwise.
    pub fn crop(&self, music: &mut Music, view: &Music, selected: &[usize]) -> Result<(), String> {
        let (p, _, (start, end)) = self.ranges(music)?;
        let events = &view
            .tracks
            .get(self.track)
            .ok_or("編集中のtrackが見つかりません")?
            .events;
        let mut keep = vec![false; events.len()];
        for &i in selected {
            *keep.get_mut(i).ok_or("eventの選択が不正です")? = true;
        }
        for (on, off) in edit::relocate::pairs(events.iter()).into_iter().enumerate() {
            if let Some(off) = off {
                keep[off] = keep[on];
            }
        }
        let kept = events
            .iter()
            .zip(keep)
            .filter(|(e, keep)| {
                *keep
                    && body::expression(e)
                    && ((start..end).contains(&e.tick) || e.tick == end && note_off(e))
            })
            .map(|(e, _)| Event {
                tick: e.tick - start,
                ..e.clone()
            })
            .collect();
        operations::crop(music, p.id, kept)
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
                .ok_or(message::MISSING_EDIT_SCENE)?;
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
        let changed = edited
            .tracks
            .get(self.track)
            .ok_or("編集中のtrackが見つかりません")?;
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
            return Err("編集が選んだpatternの本体を超えています".into());
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
