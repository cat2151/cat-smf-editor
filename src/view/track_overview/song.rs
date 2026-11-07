//! Song coordinates and stable placement origins, independent of the body shown in Notes.
use crate::app::App;
use cat_smf_editor::{
    edit::parameter_curve::Curve,
    model::Event,
    score::{musical_time::MusicalTime, pattern, scene::arrangement},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(super) struct Occurrence {
    pub pattern: u64,
    pub placement: u64,
    pub scene: Option<(u64, u64)>, // Scene ID and cell ID; placement is the enclosing Song ID.
    pub start: u64,
    pub end: u64,
    pub disabled: bool,
    pub name: String,
}

pub(super) struct Row {
    pub track: usize,
    pub occurrences: Vec<Occurrence>,
    pub events: Vec<Event>,
    pub curves: Vec<PlacedCurve>,
}

pub(super) struct PlacedCurve {
    pub curve: Curve,
    pub start: u64,
    pub end: u64,
}

pub(super) struct Song {
    pub rows: Vec<Row>,
    pub time: MusicalTime,
    pub end: u64,
}

pub(super) struct Cache {
    version: (u64, u64),
    result: Result<Arc<Song>, String>,
}

pub(super) fn enabled(a: &App) -> bool {
    a.loaded
        .as_ref()
        .and_then(|l| l.arrangement.as_ref())
        .is_some_and(|(c, _)| !c.tracks.is_empty())
}

impl Song {
    pub fn read(a: &App) -> Result<Arc<Self>, String> {
        let version = (a.song_id, a.generation);
        let mut cache = a.track_overview.cache.borrow_mut();
        if let Some(cached) = &*cache
            && cached.version == version
        {
            return cached.result.clone();
        }
        let result = Self::build(a).map(Arc::new);
        *cache = Some(Cache {
            version,
            result: result.clone(),
        });
        result
    }

    fn build(a: &App) -> Result<Self, String> {
        let l = a.loaded.as_ref().ok_or("No SMF is loaded")?;
        let (catalog, collection) = l.arrangement.as_ref().ok_or("Missing pattern catalog")?;
        let time = MusicalTime::new(&l.music);
        let map = time.map(0).ok_or("Missing Song meter")?;
        let (performed, end) = arrangement::occurrences_in(&l.music, catalog, collection, None)?;
        // Use the same validation as playback, including control conflicts.
        pattern::render::expand(&l.music, catalog, &performed, end)?;
        let mut entries = Vec::new();
        for placement in &collection.song {
            let start =
                cat_smf_editor::score::same_measure::bounds(map, placement.measure.into())?.0;
            match placement.part {
                arrangement::PatternOrScene::Pattern(id) => {
                    entries.push((id, placement.id, None, start, placement.disabled));
                }
                arrangement::PatternOrScene::Scene(id) => {
                    let s = collection
                        .scenes
                        .iter()
                        .find(|s| s.id == id)
                        .ok_or("Missing Scene")?;
                    for cell in &s.cells {
                        let offset = s.range(u128::from(cell.measure) + 1)?.0;
                        entries.push((
                            cell.pattern,
                            placement.id,
                            Some((id, cell.id)),
                            start.checked_add(offset).ok_or("Song range overflow")?,
                            placement.disabled || cell.disabled,
                        ));
                    }
                }
            }
        }
        entries.sort_by_key(|&(id, placement, scene, start, _)| (start, placement, scene, id));
        let mut tracks: Vec<_> = catalog.tracks.values().copied().collect();
        tracks.sort_unstable();
        tracks.dedup();
        let mut rows = vec![Row {
            track: 0,
            occurrences: vec![],
            events: vec![],
            curves: vec![],
        }];
        for track in tracks {
            let first = rows.len();
            rows.push(Row {
                track,
                occurrences: vec![],
                events: vec![],
                curves: vec![],
            });
            for &(id, placement, scene, start, disabled) in &entries {
                let p = catalog.patterns.get(&id).ok_or("Missing pattern")?;
                if p.track != track {
                    continue;
                }
                let end = start
                    .checked_add(p.length()?)
                    .ok_or("Song range overflow")?;
                let row = (first..rows.len())
                    .find(|&r| rows[r].occurrences.last().is_none_or(|o| o.end <= start))
                    .unwrap_or_else(|| {
                        rows.push(Row {
                            track,
                            occurrences: vec![],
                            events: vec![],
                            curves: vec![],
                        });
                        rows.len() - 1
                    });
                rows[row].occurrences.push(Occurrence {
                    pattern: id,
                    placement,
                    scene,
                    start,
                    end,
                    disabled,
                    name: p.name.clone(),
                });
            }
        }
        for row in &mut rows {
            let occurrences: Vec<_> = row
                .occurrences
                .iter()
                .filter(|o| !o.disabled)
                .map(|o| (o.pattern, o.start))
                .collect();
            let mut expansion = pattern::render::expand(&l.music, catalog, &occurrences, end)?;
            row.events = expansion.events.remove(&row.track).unwrap_or_default();
            for interval in expansion.parameters.remove(&row.track).unwrap_or_default() {
                let mut curve = interval.curve;
                for point in curve.points.iter_mut().chain(&mut curve.controls) {
                    point.tick += interval.start;
                }
                row.curves.push(PlacedCurve {
                    curve,
                    start: interval.start,
                    end: interval.end,
                });
            }
        }
        Ok(Self { rows, time, end })
    }

    pub fn measure_start(&self, tick: u64) -> u64 {
        let map = self.time.map(0).unwrap();
        map.tick_at(&super::measure(map.position(tick).measure))
            .unwrap_or(0)
    }

    pub fn at(&self, row: usize, tick: u64) -> Option<&Occurrence> {
        self.rows
            .get(row)?
            .occurrences
            .iter()
            .find(|o| (o.start..o.end).contains(&tick))
    }
}
