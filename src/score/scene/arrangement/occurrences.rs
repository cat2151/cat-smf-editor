//! Performed pattern occurrences. The Song ends with its last placement, commented out or not.
use super::*;
/// A body occurrence and its global tick offset; no saved note copies.
pub fn occurrences(m: &Music, scene: Option<u64>) -> Result<(Vec<(u64, u64)>, u64), String> {
    let catalog = Catalog::read(m)?;
    let c = read(m)?;
    occurrences_in(m, &catalog, &c, scene)
}
pub fn occurrences_in(
    m: &Music,
    catalog: &Catalog,
    c: &Collection,
    scene: Option<u64>,
) -> Result<(Vec<(u64, u64)>, u64), String> {
    let time = MusicalTime::new(m);
    let map = time.map(0).ok_or("Missing song meter")?;
    let mut out = vec![];
    let mut end = if c.song.is_empty() {
        c.song_length
    } else {
        song_end(map, c)?
    };
    if let Some(id) = scene {
        let s = c
            .scenes
            .iter()
            .find(|s| s.id == id)
            .ok_or("Missing scene")?;
        end = s.range(s.meters.len() as u128)?.1;
        for p in s.cells.iter().filter(|p| !p.disabled) {
            out.push((p.pattern, s.range(u128::from(p.measure) + 1)?.0));
        }
    } else {
        for p in &c.song {
            let start = same_measure::bounds(map, p.measure.into())?.0;
            if p.disabled {
                continue;
            }
            match p.part {
                PatternOrScene::Pattern(id) => out.push((id, start)),
                PatternOrScene::Scene(id) => {
                    let s = c
                        .scenes
                        .iter()
                        .find(|s| s.id == id)
                        .ok_or("Missing scene")?;
                    for cell in s.cells.iter().filter(|p| !p.disabled) {
                        out.push((
                            cell.pattern,
                            start + s.range(u128::from(cell.measure) + 1)?.0,
                        ));
                    }
                }
            }
        }
    }
    for &(id, _) in &out {
        if !catalog.patterns.contains_key(&id) {
            return Err("Missing pattern".into());
        }
    }
    out.sort_by_key(|&(id, tick)| (tick, id));
    Ok((out, end))
}
