//! Independent arrangement coordinates. Only pattern sound resolves to an SMF track.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    pub id: u64,
    pub name: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternOrScene {
    Pattern(u64),
    Scene(u64),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub id: u64,
    pub lane: u64,
    /// One-based Song measure.
    pub measure: u64,
    pub part: PatternOrScene,
    /// Commented out: saved and occupying its lane, but not performed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}

pub fn length(c: &Collection, part: PatternOrScene) -> Result<u64, String> {
    match part {
        PatternOrScene::Pattern(id) => c
            .pattern_lengths
            .get(&id)
            .copied()
            .ok_or("Missing pattern".into()),
        PatternOrScene::Scene(id) => c
            .scenes
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.meters.len() as u64)
            .ok_or("Missing scene".into()),
    }
}
pub fn validate(m: &Music, catalog: &Catalog, c: &Collection) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for l in &c.lanes {
        if l.id == 0 || l.name.trim().is_empty() || !ids.insert(l.id) {
            return Err("Invalid arrangement lane".into());
        }
    }
    let mut song_lanes = BTreeSet::new();
    for lane in &c.song_lanes {
        if !ids.contains(lane) || !song_lanes.insert(*lane) {
            return Err("Invalid Song lane".into());
        }
    }
    if c.song.is_empty() {
        return Ok(());
    }
    let time = MusicalTime::new(m);
    let map = time.map(0).ok_or("Arrangement requires PPQN timing")?;
    let mut occupied = vec![];
    for p in &c.song {
        if !song_lanes.contains(&p.lane) || p.measure == 0 {
            return Err("Missing placement lane or measure".into());
        }
        let end = p
            .measure
            .checked_add(length(c, p.part)?)
            .ok_or("Arrangement overflow")?;
        if occupied
            .iter()
            .any(|&(lane, start, stop)| lane == p.lane && p.measure < stop && end > start)
        {
            return Err("Arrangement blocks overlap on the same lane".into());
        }
        occupied.push((p.lane, p.measure, end));
        if same_measure::bounds(map, u128::from(end) - 1)?.1 > c.song_length {
            return Err("Placement exceeds the declared Song length".into());
        }
        match p.part {
            PatternOrScene::Pattern(id) => check_pattern(m, catalog, id, p.measure)?,
            PatternOrScene::Scene(id) => {
                let s = c
                    .scenes
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or("Missing scene")?;
                for (i, meter) in s.meters.iter().enumerate() {
                    let r = same_measure::bounds(map, u128::from(p.measure) + i as u128)?;
                    if r.1 - r.0 != meter.ticks
                        || map.signature(r.0) != (meter.numerator, meter.exponent)
                    {
                        return Err("Scene and song measure lengths differ".into());
                    }
                }
            }
        }
    }
    Ok(())
}
fn check_pattern(m: &Music, c: &Catalog, id: u64, measure: u64) -> Result<(), String> {
    let p = c.patterns.get(&id).ok_or("Missing arrangement pattern")?;
    let time = MusicalTime::new(m);
    let map = time.map(0).ok_or("Missing Song meter")?;
    for (i, meter) in p.meters.iter().enumerate() {
        let r = same_measure::bounds(map, u128::from(measure) + i as u128)?;
        if r.1 - r.0 != meter.ticks || map.signature(r.0) != (meter.numerator, meter.exponent) {
            return Err("Pattern and Song meters differ".into());
        }
    }
    Ok(())
}
fn commit(m: &mut Music, c: &Collection) -> Result<(), String> {
    let mut next = m.clone();
    let mut c = c.clone();
    let time = MusicalTime::new(m);
    let map = time.map(0).ok_or("Missing Song meter")?;
    for p in &c.song {
        c.song_length = c.song_length.max(
            same_measure::bounds(
                map,
                u128::from(p.measure) + u128::from(length(&c, p.part)?) - 1,
            )?
            .1,
        );
    }
    write(&mut next, &c)?;
    Catalog::read(&next)?;
    *m = next;
    Ok(())
}
pub fn add_lane(m: &mut Music, scene: Option<u64>) -> Result<u64, String> {
    let mut c = read(m)?;
    let id = loop {
        let id = fastrand::u64(1..);
        if !c.lanes.iter().any(|l| l.id == id) {
            break id;
        }
    };
    c.lanes.push(Lane {
        id,
        name: format!("Lane {}", c.lanes.len() + 1),
    });
    if let Some(scene) = scene {
        c.scenes
            .iter_mut()
            .find(|s| s.id == scene)
            .ok_or("Missing scene")?
            .lanes
            .push(id);
    } else {
        c.song_lanes.push(id);
    }
    commit(m, &c)?;
    Ok(id)
}
pub fn create_scene(m: &mut Music, name: &str, measures: u64, lanes: usize) -> Result<u64, String> {
    if name.trim().is_empty() || measures == 0 || lanes == 0 || measures > 100_000 || lanes > 1024 {
        return Err("Enter a name, positive length and lane count".into());
    }
    let mut c = read(m)?;
    let time = MusicalTime::new(m);
    let map = time.map(0).ok_or("Missing meter")?;
    let mut meters = vec![];
    for i in 1..=measures {
        let r = same_measure::bounds(map, i.into())?;
        let (numerator, exponent) = map.signature(r.0);
        meters.push(Meter {
            numerator,
            exponent,
            ticks: r.1 - r.0,
        });
    }
    let id = loop {
        let id = fastrand::u64(1..);
        if !c.scenes.iter().any(|s| s.id == id) {
            break id;
        }
    };
    let mut ids = vec![];
    for i in 0..lanes {
        let lane = loop {
            let id = fastrand::u64(1..);
            if !c.lanes.iter().any(|l| l.id == id) {
                break id;
            }
        };
        c.lanes.push(Lane {
            id: lane,
            name: format!("Lane {}", i + 1),
        });
        ids.push(lane);
    }
    c.scenes.push(Scene {
        id,
        name: name.trim().into(),
        meters,
        lanes: ids,
        cells: vec![],
    });
    commit(m, &c)?;
    Ok(id)
}
/// Replace only an explicitly selected cell. Other intersecting blocks reject atomically.
pub fn place(
    m: &mut Music,
    scene: Option<u64>,
    lane: u64,
    measure: u64,
    part: Option<PatternOrScene>,
) -> Result<(), String> {
    let mut c = read(m)?;
    if let Some(id) = scene {
        let s = c
            .scenes
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Missing scene")?;
        if measure == 0 || !s.lanes.contains(&lane) {
            return Err("Missing scene cell".into());
        }
        s.range(measure.into())?;
        let placement_id = s
            .cells
            .iter()
            .find(|p| p.lane == lane && p.measure == measure - 1)
            .map_or_else(|| fastrand::u64(1..), |p| p.id);
        s.cells
            .retain(|cell| cell.lane != lane || cell.measure != measure - 1);
        if let Some(part) = part {
            let PatternOrScene::Pattern(pattern) = part else {
                return Err("Scenes contain patterns only".into());
            };
            s.cells.push(Cell {
                id: placement_id,
                lane,
                measure: measure - 1,
                pattern,
                disabled: false,
            });
        }
    } else {
        let placement_id = c
            .song
            .iter()
            .find(|p| p.lane == lane && p.measure == measure)
            .map_or_else(|| fastrand::u64(1..), |p| p.id);
        c.song.retain(|p| p.lane != lane || p.measure != measure);
        if let Some(part) = part {
            c.song.push(Placement {
                id: placement_id,
                lane,
                measure,
                part,
                disabled: false,
            });
        }
    }
    commit(m, &c)
}
pub fn move_part(
    m: &mut Music,
    scene: Option<u64>,
    from: (u64, u64),
    to: (u64, u64),
) -> Result<(), String> {
    let c = read(m)?;
    let part = cell(&c, scene, from.0, from.1).ok_or("Empty source cell")?;
    if cell(&c, scene, to.0, to.1).is_some() {
        return Err("Destination is occupied".into());
    }
    let mut c = c;
    if let Some(id) = scene {
        let s = c
            .scenes
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Missing scene")?;
        if !s.lanes.contains(&to.0) || to.1 == 0 {
            return Err("Invalid destination lane".into());
        }
        s.range(to.1.into())?;
        let p = s
            .cells
            .iter_mut()
            .find(|p| p.lane == from.0 && p.measure + 1 == from.1)
            .ok_or("Missing placement")?;
        p.lane = to.0;
        p.measure = to.1 - 1;
    } else {
        if !c.song_lanes.contains(&to.0) || to.1 == 0 {
            return Err("Invalid destination lane".into());
        }
        let p = c
            .song
            .iter_mut()
            .find(|p| p.lane == from.0 && p.measure == from.1)
            .ok_or("Missing placement")?;
        p.lane = to.0;
        p.measure = to.1;
    }
    let _ = part;
    commit(m, &c)?;
    Ok(())
}
pub fn delete_scene(m: &mut Music, id: u64) -> Result<(), String> {
    let mut c = read(m)?;
    if c.song.iter().any(|p| p.part == PatternOrScene::Scene(id)) {
        return Err("Scene is in use".into());
    }
    if !c.scenes.iter().any(|s| s.id == id) {
        return Err("Missing scene".into());
    }
    let lanes = c.scenes.iter().find(|s| s.id == id).unwrap().lanes.clone();
    c.lanes.retain(|l| !lanes.contains(&l.id));
    c.scenes.retain(|s| s.id != id);
    commit(m, &c)
}
pub fn cell(c: &Collection, scene: Option<u64>, lane: u64, measure: u64) -> Option<PatternOrScene> {
    if let Some(id) = scene {
        c.scenes
            .iter()
            .find(|s| s.id == id)?
            .cells
            .iter()
            .find(|p| p.lane == lane && p.measure + 1 == measure)
            .map(|p| PatternOrScene::Pattern(p.pattern))
    } else {
        c.song
            .iter()
            .find(|p| p.lane == lane && p.measure == measure)
            .map(|p| p.part)
    }
}
/// The full selected block, including a measure inside a multimeasure body.
pub fn covering(
    c: &Collection,
    scene: Option<u64>,
    lane: u64,
    measure: u64,
) -> Option<(PatternOrScene, u64)> {
    if let Some(id) = scene {
        c.scenes
            .iter()
            .find(|s| s.id == id)?
            .cells
            .iter()
            .find_map(|p| {
                let start = p.measure + 1;
                let n = *c.pattern_lengths.get(&p.pattern)?;
                (p.lane == lane && start <= measure && measure < start.checked_add(n)?)
                    .then_some((PatternOrScene::Pattern(p.pattern), start))
            })
    } else {
        c.song.iter().find_map(|p| {
            (p.lane == lane
                && p.measure <= measure
                && measure < p.measure.checked_add(length(c, p.part).ok()?)?)
            .then_some((p.part, p.measure))
        })
    }
}
/// Where a measure of the song, or of a scene, begins.
pub fn measure_tick(m: &Music, scene: Option<u64>, measure: u64) -> Result<u64, String> {
    if let Some(id) = scene {
        let c = read(m)?;
        let s = c
            .scenes
            .iter()
            .find(|s| s.id == id)
            .ok_or("Missing scene")?;
        return Ok(s.range(measure.into())?.0);
    }
    let time = MusicalTime::new(m);
    Ok(same_measure::bounds(time.map(0).ok_or("Missing Song meter")?, measure.into())?.0)
}
/// Comment out, or restore, the placement covering a cell. Restoring rejects atomically when
/// the placement would conflict with what is already performed.
pub fn toggle(m: &mut Music, scene: Option<u64>, lane: u64, measure: u64) -> Result<(), String> {
    let mut c = read(m)?;
    let (_, start) = covering(&c, scene, lane, measure).ok_or("Empty cell")?;
    let disabled = if let Some(id) = scene {
        &mut c
            .scenes
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("Missing scene")?
            .cells
            .iter_mut()
            .find(|p| p.lane == lane && p.measure + 1 == start)
            .ok_or("Missing placement")?
            .disabled
    } else {
        &mut c
            .song
            .iter_mut()
            .find(|p| p.lane == lane && p.measure == start)
            .ok_or("Missing placement")?
            .disabled
    };
    *disabled ^= true;
    commit(m, &c)
}
/// Whether the placement covering a cell is commented out.
pub fn commented_out(c: &Collection, scene: Option<u64>, lane: u64, measure: u64) -> bool {
    let Some((_, start)) = covering(c, scene, lane, measure) else {
        return false;
    };
    if let Some(id) = scene {
        c.scenes.iter().find(|s| s.id == id).is_some_and(|s| {
            s.cells
                .iter()
                .any(|p| p.lane == lane && p.measure + 1 == start && p.disabled)
        })
    } else {
        c.song
            .iter()
            .any(|p| p.lane == lane && p.measure == start && p.disabled)
    }
}
mod occurrences;
pub use occurrences::{occurrences, occurrences_in};

#[cfg(test)]
mod tests;
