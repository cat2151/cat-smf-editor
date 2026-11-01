use super::*;
use crate::message;
use crate::{edit, model::Track};

pub fn create_source(music: &mut Music) -> Result<usize, String> {
    let id = Catalog::read(music)?.fresh_id();
    let track = music.tracks.len();
    music.tracks.push(Track {
        events: vec![
            Event {
                tick: 0,
                delta: 0,
                status: 0xff,
                meta_type: 3,
                data: b"Init Saw".to_vec(),
            },
            event(
                0,
                &Record::Container(ContainerRecord {
                    version: VERSION,
                    id,
                }),
            ),
        ],
    });
    if music.format == 0 {
        music.format = 1;
    }
    Ok(track)
}
/// The storage tick after every pattern on `track`.
pub(super) fn free_tick(c: &Catalog, track: usize) -> Result<u64, String> {
    c.patterns
        .values()
        .filter(|p| p.track == track)
        .map(|p| {
            p.range()
                .and_then(|r| r.1.checked_add(1).ok_or(message::STORAGE_OVERFLOW.into()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|ends| ends.into_iter().max().unwrap_or(0))
}
fn commit(music: &mut Music, next: Music) -> Result<(), String> {
    Catalog::read(&next)?;
    *music = next;
    Ok(())
}
pub fn create(music: &mut Music, source: usize, duplicate: Option<u64>) -> Result<u64, String> {
    let c = Catalog::read(music)?;
    if !c.sources.contains(&source) {
        return Err("sound containerを選んでください".into());
    }
    let tick = free_tick(&c, source)?;
    let id = c.fresh_id();
    let (kind, name, meters, mut events) = if let Some(original) = duplicate {
        let p = c.patterns.get(&original).ok_or(message::MISSING_PATTERN)?;
        if p.track != source {
            return Err("複製は元のcontainerのままです。後から移動してください".into());
        }
        let mut events = body::extract(&music.tracks[p.track].events, p.range()?)?;
        for e in &mut events {
            e.tick = tick
                .checked_add(e.tick - p.tick)
                .ok_or(message::STORAGE_OVERFLOW)?;
        }
        (p.kind, format!("{} copy", p.name), p.meters.clone(), events)
    } else {
        let ticks = u64::from(music.division)
            .checked_mul(4)
            .filter(|_| music.division & 0x8000 == 0 && music.division > 0)
            .ok_or(message::PATTERN_REQUIRES_PPQN_TIMING)?;
        (
            Kind::Normal,
            format!("Pattern {}", c.patterns.len() + 1),
            vec![Meter {
                numerator: 4,
                exponent: 2,
                ticks,
            }],
            vec![],
        )
    };
    events.push(event(
        tick,
        &Record::Pattern(PatternRecord {
            version: VERSION,
            id,
            kind,
            name,
            meters,
        }),
    ));
    let mut next = music.clone();
    for e in events {
        edit::insert(&mut next.tracks[source], e);
    }
    edit::refresh_deltas(&mut next.tracks[source])?;
    commit(music, next)?;
    Ok(id)
}
/// An empty one-bar Capture pattern named `Capture`.
pub fn create_capture(music: &mut Music, source: usize) -> Result<u64, String> {
    let id = create(music, source, None)?;
    let meta = music.tracks[source]
        .events
        .iter_mut()
        .find(|e| matches!(record(e), Ok(Some(Record::Pattern(r))) if r.id == id))
        .ok_or(message::MISSING_DEFINITION)?;
    let Ok(Some(Record::Pattern(mut r))) = record(meta) else {
        return Err(message::MISSING_DEFINITION.into());
    };
    r.kind = Kind::Capture;
    r.name = "Capture".into();
    *meta = event(meta.tick, &Record::Pattern(r));
    Ok(id)
}
pub fn delete(music: &mut Music, id: u64) -> Result<(), String> {
    let c = Catalog::read(music)?;
    let collection = crate::score::scene::read(music)?;
    if collection
        .song
        .iter()
        .any(|p| p.part == crate::score::scene::arrangement::PatternOrScene::Pattern(id))
        || collection
            .scenes
            .iter()
            .any(|s| s.cells.iter().any(|cell| cell.pattern == id))
    {
        return Err("patternは使用中です".into());
    }
    let p = c.patterns.get(&id).ok_or(message::MISSING_PATTERN)?;
    let mut next = music.clone();
    remove(&mut next.tracks[p.track], p)?;
    commit(music, next)
}
pub(super) fn remove(track: &mut Track, p: &Definition) -> Result<(), String> {
    let range = p.range()?;
    track.events.retain(|e| {
        !(body::expression(e) && (range.0..=range.1).contains(&e.tick))
            && !matches!(record(e),Ok(Some(Record::Pattern(r))) if r.id == p.id)
    });
    edit::refresh_deltas(track)
}
/// Change storage location or ownership without changing the pattern ID or event bytes.
pub fn relocate(music: &mut Music, id: u64, container: u64, tick: u64) -> Result<(), String> {
    let c = Catalog::read(music)?;
    let p = c.patterns.get(&id).ok_or(message::MISSING_PATTERN)?;
    let dest = *c
        .tracks
        .get(&container)
        .filter(|t| c.sources.contains(t))
        .ok_or(message::MISSING_CONTAINER)?;
    if p.track != dest
        && crate::score::drums::read(&music.tracks[p.track])?.is_some()
            != crate::score::drums::read(&music.tracks[dest])?.is_some()
    {
        return Err("patternのdrumの行の解釈が移動先のcontainerに合いません".into());
    }
    let mut events = body::extract(&music.tracks[p.track].events, p.range()?)?;
    super::automation::retarget(&mut events, container)?;
    for e in &mut events {
        e.tick = tick
            .checked_add(e.tick - p.tick)
            .ok_or(message::STORAGE_OVERFLOW)?;
    }
    events.push(event(
        tick,
        &Record::Pattern(PatternRecord {
            version: VERSION,
            id,
            kind: p.kind,
            name: p.name.clone(),
            meters: p.meters.clone(),
        }),
    ));
    let mut next = music.clone();
    remove(&mut next.tracks[p.track], p)?;
    for e in events {
        edit::insert(&mut next.tracks[dest], e);
    }
    edit::refresh_deltas(&mut next.tracks[dest])?;
    if container != p.container {
        let index = Catalog::read(&next)?;
        super::render::expand(&next, &index, &[(id, 0)], p.length()?)?;
    }
    commit(music, next)
}
pub fn resize(music: &mut Music, id: u64, meters: Vec<Meter>) -> Result<(), String> {
    let c = Catalog::read(music)?;
    let p = c.patterns.get(&id).ok_or(message::MISSING_PATTERN)?;
    body::meter_music(&meters, music.division)?;
    let length = body::length(&meters)?;
    if body::extract(&music.tracks[p.track].events, p.range()?)?
        .iter()
        .any(|e| {
            e.tick - p.tick > length
                || (e.tick - p.tick == length && !crate::score::note_view::note_off(e))
        })
    {
        return Err("patternの中身が指定の長さを超えています".into());
    }
    let mut next = music.clone();
    let end = p
        .tick
        .checked_add(length)
        .ok_or(message::PATTERN_RANGE_OVERFLOW)?;
    if c.patterns.values().any(|other| {
        other.id != id
            && other.track == p.track
            && other.range().is_ok_and(|r| p.tick <= r.1 && end >= r.0)
    }) {
        relocate(&mut next, id, p.container, free_tick(&c, p.track)?)?;
    }
    let meta = next.tracks[p.track]
        .events
        .iter_mut()
        .find(|e| matches!(record(e),Ok(Some(Record::Pattern(r))) if r.id == id))
        .ok_or(message::MISSING_DEFINITION)?;
    *meta = event(
        meta.tick,
        &Record::Pattern(PatternRecord {
            version: VERSION,
            id,
            kind: p.kind,
            name: p.name.clone(),
            meters,
        }),
    );
    let collection = crate::score::scene::read(&next)?;
    if collection.song.iter().any(|placement| {
        placement.part == crate::score::scene::arrangement::PatternOrScene::Pattern(id)
    }) {
        crate::score::scene::arrangement::commit(&mut next, &collection)?;
    }
    commit(music, next)
}
