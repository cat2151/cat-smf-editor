use super::Curve;
use crate::{
    edit::{insert, refresh_deltas},
    model::{Event, Music, Track},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GROUP_PREFIX: &[u8] = b"\x7dcat-smf-editor\0instance-group\0";
pub const PREFIX: &[u8] = b"\x7dcat-smf-editor\0parameter-curve\0";
const MAX_PAYLOAD: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub version: u8,
    pub id: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Collection {
    pub owners: BTreeMap<u64, usize>,
    pub curves: BTreeMap<usize, Curve>,
}
impl Collection {
    pub fn owner(&self, track: usize) -> Option<usize> {
        self.curves
            .get(&track)
            .and_then(|curve| self.owners.get(&curve.target.group))
            .copied()
    }
    /// Empty curves never conflict. Every conflicting track is reported.
    pub fn conflicts(&self) -> Vec<Vec<usize>> {
        let mut targets: BTreeMap<_, Vec<usize>> = BTreeMap::new();
        for (&track, curve) in &self.curves {
            if !curve.points.is_empty() {
                targets.entry(&curve.target).or_default().push(track);
            }
        }
        targets
            .into_values()
            .filter(|tracks| tracks.len() > 1)
            .collect()
    }
}

fn meta(event: &Event, prefix: &[u8]) -> bool {
    event.status == 0xff && event.meta_type == 0x7f && event.data.starts_with(prefix)
}
pub fn owned(event: &Event) -> bool {
    meta(event, PREFIX)
}
pub fn group_event(event: &Event) -> bool {
    meta(event, GROUP_PREFIX)
}
fn event(tick: u64, kind: u8, data: Vec<u8>) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: kind,
        data,
    }
}
fn payload<T: Serialize>(prefix: &[u8], value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = prefix.to_vec();
    bytes.extend(serde_json::to_vec(value).map_err(|e| e.to_string())?);
    if bytes.len() > MAX_PAYLOAD {
        return Err("Parameter metadata is too large".into());
    }
    Ok(bytes)
}

pub fn read(music: &Music) -> Result<Collection, String> {
    let mut result = Collection::default();
    for (track, source) in music.tracks.iter().enumerate() {
        let mut group = None;
        for e in &source.events {
            if !owned(e) && !group_event(e) {
                continue;
            }
            if music.format != 1 || e.tick != 0 || e.data.len() > MAX_PAYLOAD {
                return Err("Parameter metadata requires tick zero in SMF format 1".into());
            }
            if group_event(e) {
                let value: Group = serde_json::from_slice(&e.data[GROUP_PREFIX.len()..])
                    .map_err(|e| e.to_string())?;
                if value.version != 1
                    || value.id == 0
                    || group.replace(value.id).is_some()
                    || result.owners.insert(value.id, track).is_some()
                {
                    return Err("Invalid or duplicate instance group identity".into());
                }
            } else {
                let curve: Curve =
                    serde_json::from_slice(&e.data[PREFIX.len()..]).map_err(|e| e.to_string())?;
                curve.validate()?;
                if track == 0 || result.curves.insert(track, curve).is_some() {
                    return Err(
                        "A dedicated parameter track needs one curve outside the conductor".into(),
                    );
                }
            }
        }
        if result.curves.contains_key(&track)
            && (group.is_some()
                || source
                    .events
                    .iter()
                    .any(|e| matches!(e.status & 0xf0, 0x80 | 0x90)))
        {
            return Err("A parameter track cannot also own an instrument or notes".into());
        }
    }
    Ok(result)
}

/// Caller has resolved and revalidated the selected plugin/parameter on the UI thread.
/// `curve.target.group` is assigned here; existing matching targets are reused.
pub fn create(music: &mut Music, owner: usize, mut curve: Curve) -> Result<usize, String> {
    let saved = read(music)?;
    if music.format != 1 || owner >= music.tracks.len() || saved.curves.contains_key(&owner) {
        return Err("Automation requires a format 1 instrument owner track".into());
    }
    let group = saved
        .owners
        .iter()
        .find_map(|(&id, &track)| (track == owner).then_some(id));
    let id = match group {
        Some(id) => id,
        None => saved
            .owners
            .keys()
            .copied()
            .chain(saved.curves.values().map(|c| c.target.group))
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Instance group IDs exhausted")?,
    };
    curve.target.group = id;
    curve.validate()?;
    let mut matches = saved
        .curves
        .iter()
        .filter(|(_, c)| c.target == curve.target);
    if let Some((&track, _)) = matches.next() {
        if matches.next().is_some() {
            return Err("Multiple tracks share this parameter target".into());
        }
        return Ok(track);
    }
    if music.tracks.len() >= 32767 {
        return Err("Too many SMF tracks".into());
    }
    let mut next = music.clone();
    if group.is_none() {
        insert(
            &mut next.tracks[owner],
            event(0, 0x7f, payload(GROUP_PREFIX, &Group { version: 1, id })?),
        );
        refresh_deltas(&mut next.tracks[owner])?;
    }
    let end = next.tracks[owner]
        .events
        .last()
        .map_or(0, |e| e.tick)
        .max(curve.points.last().map_or(0, |p| p.tick));
    let name = if curve.path.is_empty() {
        curve.name.clone()
    } else {
        format!("{} / {}", curve.path, curve.name)
    };
    next.tracks.push(Track {
        events: vec![
            event(0, 0x7f, payload(PREFIX, &curve)?),
            event(0, 0x03, name.into_bytes()),
            event(end, 0x2f, vec![]),
        ],
    });
    let index = next.tracks.len() - 1;
    refresh_deltas(&mut next.tracks[index])?;
    read(&next)?;
    crate::smf_io::encode(&next)?;
    *music = next;
    Ok(index)
}

pub fn write(music: &mut Music, track: usize, curve: &Curve) -> Result<(), String> {
    let saved = read(music)?;
    let old = saved.curves.get(&track).ok_or("Not a parameter track")?;
    if old.target != curve.target {
        return Err("Parameter target changed during editing".into());
    }
    curve.validate()?;
    let data = payload(PREFIX, curve)?;
    let mut next = music.clone();
    let source = &mut next.tracks[track];
    source.events.retain(|e| !owned(e));
    insert(source, event(0, 0x7f, data));
    let end = curve.points.last().map_or(0, |p| p.tick);
    if let Some(eot) = source
        .events
        .last_mut()
        .filter(|e| e.status == 0xff && e.meta_type == 0x2f)
    {
        eot.tick = eot.tick.max(end);
    } else {
        source.events.push(event(
            source.events.last().map_or(end, |e| e.tick.max(end)),
            0x2f,
            vec![],
        ));
    }
    refresh_deltas(source)?;
    read(&next)?;
    crate::smf_io::encode(&next)?;
    *music = next;
    Ok(())
}
