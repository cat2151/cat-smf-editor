//! Lanes, Scenes and the Song live in one conductor JSON meta; positions are measure numbers.
use super::*;
pub const VERSION: u8 = 3;
pub fn read(m: &Music) -> Result<Collection, String> {
    let mut found = None;
    for (track, t) in m.tracks.iter().enumerate() {
        for e in t.events.iter().filter(|e| super::owned(e)) {
            if track != 0 || e.tick != 0 || found.is_some() {
                return Err("Duplicate or misplaced scene metadata".into());
            }
            found = Some(
                serde_json::from_slice::<Collection>(&e.data[PREFIX.len()..])
                    .map_err(|e| format!("Invalid scene metadata: {e}"))?,
            );
        }
    }
    let mut c = found.unwrap_or_default();
    if c.version != VERSION {
        return Err("Invalid scene version".into());
    }
    for t in &m.tracks {
        for e in &t.events {
            if let Some(crate::score::pattern::Record::Definition { id, meters, .. }) =
                crate::score::pattern::record(e)?
            {
                c.pattern_lengths.insert(id, meters.len() as u64);
            }
        }
    }
    let mut ids = BTreeSet::new();
    for id in c
        .scenes
        .iter()
        .map(|s| s.id)
        .chain(c.lanes.iter().map(|l| l.id))
        .chain(c.scenes.iter().flat_map(|s| s.cells.iter().map(|p| p.id)))
        .chain(c.song.iter().map(|p| p.id))
    {
        if id == 0 || !ids.insert(id) {
            return Err("Duplicate or zero arrangement identity".into());
        }
    }
    for l in &c.lanes {
        let owners = c.scenes.iter().filter(|s| s.lanes.contains(&l.id)).count()
            + usize::from(c.song_lanes.contains(&l.id));
        if owners != 1 {
            return Err("Lane requires exactly one owner".into());
        }
    }
    for s in &c.scenes {
        if s.cells.iter().any(|p| !s.lanes.contains(&p.lane)) {
            return Err("Scene placement outside its lanes".into());
        }
    }
    if c.song
        .iter()
        .any(|p| !c.song_lanes.contains(&p.lane) || p.measure == 0)
    {
        return Err("Invalid Song placement".into());
    }
    // Presentation order does not depend on edit history.
    c.lanes.sort_by_key(|l| l.id);
    for s in &mut c.scenes {
        s.cells.sort_by_key(|p| (p.lane, p.measure));
    }
    c.song.sort_by_key(|p| (p.lane, p.measure));
    Ok(c)
}
pub fn write(m: &mut Music, c: &Collection) -> Result<(), String> {
    let mut next = m.clone();
    if next.tracks.is_empty() {
        return Err("Missing conductor track".into());
    }
    next.tracks[0].events.retain(|e| !super::owned(e));
    let data = [
        PREFIX,
        serde_json::to_vec(c).map_err(|e| e.to_string())?.as_slice(),
    ]
    .concat();
    crate::edit::insert(
        &mut next.tracks[0],
        Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 0x7f,
            data,
        },
    );
    crate::edit::refresh_deltas(&mut next.tracks[0])?;
    // Persistence must be atomic even for raw editors and callers outside arrangement operations.
    read(&next)?;
    Catalog::read(&next)?;
    *m = next;
    Ok(())
}
