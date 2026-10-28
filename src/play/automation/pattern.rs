//! Pattern expression windows stay discontinuous; gaps use the container's baseline.
use super::*;
use cat_smf_editor::score::pattern::render::ParameterInterval;
pub(super) fn audition_notes(
    app: &mut App,
    tick: u64,
    notes: &[(u8, u8, Option<f64>)],
    take: Option<Arc<crate::play::sung::Take>>,
) -> bool {
    let id = app.patterns.editing.as_ref().unwrap().pattern;
    let mut prepared = match crate::view::patterns::preview::prepare(app, id) {
        Ok(p) => p,
        Err(error) => {
            app.audition.stop();
            app.load_error = Some(error);
            return true;
        }
    };
    if !prepared.active.automation.active() {
        return false;
    }
    let (track, patch, mut body) = prepared.parts.remove(0);
    let origin = prepared.active.tempos[track].seconds_at(tick);
    body.skip(origin);
    let notes = notes
        .iter()
        .map(
            |&(pitch, velocity, duration)| crate::play::audio_sequence::Note {
                start: 0.,
                duration: duration.unwrap_or(f64::INFINITY),
                pitch,
                velocity,
            },
        )
        .collect::<Vec<_>>();
    let end = notes.iter().map(|n| n.duration).fold(0., f64::max);
    let mut sequence = Sequence::new(notes, end);
    sequence.origin = origin;
    for e in &mut sequence.events {
        e.time += origin;
    }
    let mut controls = body
        .events
        .into_iter()
        .filter(|e| e.time == origin && matches!((e.word >> 20) & 15, 11 | 14))
        .collect::<Vec<_>>();
    controls.extend(sequence.events);
    sequence.events = controls;
    #[cfg(test)]
    {
        app.audition.auditioned = sequence.notes.iter().map(|n| n.pitch).collect();
    }
    app.audition.sequences_automated(
        vec![(track, patch, sequence)],
        take.map(|t| vec![(track, t)]).unwrap_or_default(),
        &prepared.active.automation,
        origin,
    );
    true
}
fn same(a: &parameter_curve::Target, b: &parameter_curve::Target) -> bool {
    a.layer == b.layer
        && a.plugin_format == b.plugin_format
        && a.plugin_id == b.plugin_id
        && a.parameter_id == b.parameter_id
}
pub(crate) fn prepare(
    app: &App,
    parts: &[(usize, Patch, Sequence)],
    intervals: &BTreeMap<usize, Vec<ParameterInterval>>,
    events: &BTreeMap<usize, Vec<cat_smf_editor::model::Event>>,
    tempo: &TempoMap,
) -> Result<Automation, String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let mut owners = vec![];
    for (owner, patch, sequence) in parts {
        let mut curves: Vec<Arc<Curve>> = vec![];
        let mut windows: BTreeMap<_, Vec<ParameterInterval>> = BTreeMap::new();
        for interval in intervals.get(owner).into_iter().flatten() {
            windows
                .entry(interval.curve.target.clone())
                .or_default()
                .push(interval.clone());
            if !curves.iter().any(|c| c.target == interval.curve.target) {
                curves.push(Arc::new(interval.curve.clone()));
            }
        }
        let fixed =
            cat_smf_editor::score::macro_values::read(&loaded.music.tracks[*owner])?.parameters;
        let live = crate::view::macro_editor::live(app, *owner);
        let mut resolved_live = BTreeMap::new();
        for curve in &curves {
            if let Some((_, value)) = live.iter().find(|(target, _)| same(target, &curve.target)) {
                resolved_live.insert(curve.target.clone(), value.clone());
            }
        }
        for c in fixed {
            let layer = patch
                .layers
                .iter()
                .find(|l| l.settings.id == c.target.layer)
                .ok_or("Missing container macro layer")?;
            let key = layer
                .instrument
                .as_ref()
                .map(|s| Ok(s.plugin.clone()))
                .unwrap_or_else(cat_smf_editor::chain::default_instrument)?;
            if key.id != c.target.plugin_id || key.format != c.target.plugin_format {
                return Err("Container macro instrument changed".into());
            }
            if let Some(curve) = curves.iter().find(|curve| same(&curve.target, &c.target)) {
                let value = c.value_at(0.0).ok_or("Missing fixed macro baseline")?;
                if c.value_space != curve.value_space
                    || c.minimum != curve.minimum
                    || c.maximum != curve.maximum
                {
                    return Err("Container macro baseline domain changed".into());
                }
                for w in windows.get_mut(&curve.target).into_iter().flatten() {
                    w.baseline = value;
                }
            } else {
                if let Some(value) = live.get(&c.target) {
                    resolved_live.insert(c.target.clone(), value.clone());
                }
                curves.push(Arc::new(c));
            }
        }
        let midi = sequence
            .events
            .iter()
            .any(|e| matches!((e.word >> 20) & 15, 11 | 14));
        if curves.is_empty() && !midi {
            continue;
        }
        let mut ticks = events
            .get(owner)
            .into_iter()
            .flatten()
            .map(|e| e.tick)
            .collect::<Vec<_>>();
        for values in windows.values_mut() {
            values.sort_by_key(|w| w.start);
            for w in values {
                ticks.push(w.start);
                ticks.push(w.end);
                for p in &w.curve.points {
                    ticks.push(
                        w.start
                            .checked_add(p.tick)
                            .ok_or("Pattern curve time overflow")?,
                    );
                }
            }
        }
        ticks.sort_unstable();
        ticks.dedup();
        owners.push(Arc::new(OwnerPlan {
            windows: windows
                .into_iter()
                .map(|(target, w)| (target, Arc::new(w)))
                .collect(),
            midi,
            live: resolved_live,
            owner: *owner,
            curves,
            tempo: Arc::new(tempo.clone()),
            boundaries: ticks
                .into_iter()
                .map(|tick| (tick, tempo.seconds_at(tick)))
                .collect(),
        }));
    }
    Ok(Automation {
        owners: Arc::new(owners),
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use parameter_curve::{Point, Target, ValueSpace};
    #[test]
    fn windows_restore_baseline_in_gaps_switch_adjacent_and_rebase_after_seek() {
        let curve = Curve {
            version: 1,
            target: Target {
                group: 1,
                layer: 1,
                plugin_format: "CLAP".into(),
                plugin_id: "fixture".into(),
                parameter_id: "101".into(),
            },
            value_space: ValueSpace::Plain,
            minimum: 0.,
            maximum: 1.,
            name: "Cutoff".into(),
            path: "Filter".into(),
            points: vec![
                Point {
                    tick: 0,
                    value: 0.2,
                },
                Point {
                    tick: 100,
                    value: 0.8,
                },
            ],
            controls: vec![Point {
                tick: 50,
                value: 0.5,
            }],
        };
        curve.validate().unwrap();
        let interval = |start, end| ParameterInterval {
            start,
            end,
            baseline: 0.4,
            curve: curve.clone(),
        };
        let binding = Binding {
            windows: Arc::new(vec![
                interval(100, 200),
                interval(200, 300),
                interval(400, 500),
            ]),
            live: None,
            curve: Arc::new(curve),
            value: Default::default(),
        };
        assert_eq!(binding.at(0.), Some(0.4));
        assert!((binding.at(150.).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(binding.at(200.), Some(0.2));
        assert_eq!(binding.at(300.), Some(0.4));
        assert_eq!(binding.at(400.), Some(0.2));
        assert_eq!(binding.at(500.), Some(0.4));
        assert_eq!(binding.at(100.), Some(0.2));
    }
}
