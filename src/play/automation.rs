//! UI-prepared owner plans and audio sample boundaries. No native discovery in render.
use crate::{app::App, play::audio_sequence::Sequence};
use cat_smf_editor::{
    edit::parameter_curve::{self, Curve},
    score::{audition_timing::TempoMap, automation, instrument::Patch},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct Binding {
    pub windows: Arc<Vec<cat_smf_editor::score::pattern::render::ParameterInterval>>,
    pub live: Option<Arc<std::sync::atomic::AtomicU64>>,
    pub curve: Arc<Curve>,
    pub value: cat_smf_editor::host::parameters::ParameterValue,
}
impl Binding {
    pub fn at(&self, tick: f64) -> Option<f64> {
        if !self.windows.is_empty() {
            let index = self.windows.partition_point(|w| w.start as f64 <= tick);
            if let Some(w) = index
                .checked_sub(1)
                .and_then(|i| self.windows.get(i))
                .filter(|w| tick < w.end as f64)
            {
                return w.curve.value_at(tick - w.start as f64);
            }
            return self
                .live
                .as_ref()
                .map(|v| f64::from_bits(v.load(std::sync::atomic::Ordering::Relaxed)))
                .or_else(|| self.windows.first().map(|w| w.baseline));
        }
        self.live
            .as_ref()
            .map(|v| f64::from_bits(v.load(std::sync::atomic::Ordering::Relaxed)))
            .or_else(|| self.curve.value_at(tick))
    }
}
#[derive(Clone)]
pub struct LayerBindings {
    pub layer: usize,
    pub lanes: Vec<Binding>,
    pub values: Vec<cat_smf_editor::host::parameters::ParameterValue>,
}

#[derive(Clone)]
pub struct OwnerPlan {
    pub windows: BTreeMap<
        parameter_curve::Target,
        Arc<Vec<cat_smf_editor::score::pattern::render::ParameterInterval>>,
    >,
    pub midi: bool,
    pub live: BTreeMap<parameter_curve::Target, Arc<std::sync::atomic::AtomicU64>>,
    pub owner: usize,
    pub curves: Vec<Arc<Curve>>,
    pub tempo: Arc<TempoMap>,
    pub boundaries: Vec<(u64, f64)>,
}

#[derive(Clone, Default)]
pub struct Automation {
    pub owners: Arc<Vec<Arc<OwnerPlan>>>,
    pub error: Option<String>,
}
impl Automation {
    pub fn prepare(app: &App, parts: &[(usize, Patch, Sequence)], all: bool) -> Self {
        Self::scoped(app, parts, all, &[app.track])
    }
    pub fn scoped(
        app: &App,
        parts: &[(usize, Patch, Sequence)],
        all: bool,
        selected: &[usize],
    ) -> Self {
        match prepare(app, parts, all, selected) {
            Ok(owners) => Self {
                owners: Arc::new(owners),
                error: None,
            },
            Err(error) => Self {
                error: Some(error),
                ..Self::default()
            },
        }
    }
    pub fn owner(&self, owner: usize) -> Option<Arc<OwnerPlan>> {
        self.owners.iter().find(|plan| plan.owner == owner).cloned()
    }
    pub fn active(&self) -> bool {
        !self.owners.is_empty()
    }
}

fn prepare(
    app: &App,
    parts: &[(usize, Patch, Sequence)],
    all: bool,
    selected: &[usize],
) -> Result<Vec<Arc<OwnerPlan>>, String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let collection = parameter_curve::read(&loaded.music)?;
    let patches: Vec<_> = parts
        .iter()
        .map(|(track, patch, _)| (*track, patch.clone()))
        .collect();
    let mut owners: BTreeMap<usize, Vec<Arc<Curve>>> = BTreeMap::new();
    for (&track, curve) in &collection.curves {
        if curve.points.is_empty() {
            continue;
        }
        let owner = collection.owner(track);
        let included = all
            || selected.contains(&track)
            || owner.is_some_and(|o| parts.iter().any(|p| p.0 == o));
        if !included {
            continue;
        }
        let owner =
            owner.ok_or_else(|| format!("Missing owner: {} / {}", curve.path, curve.name))?;
        if cat_smf_editor::voicevox::is_singer(&loaded.music.tracks[owner]) {
            return Err(format!(
                "VOICEVOX cannot automate plugin parameters: {}",
                curve.name
            ));
        }
        let patch = patches
            .iter()
            .find_map(|(t, p)| (*t == owner).then_some(p))
            .ok_or("Automation owner is outside playback scope")?;
        let fallback = if patch
            .layers
            .iter()
            .any(|l| l.settings.id == curve.target.layer && l.instrument.is_none())
        {
            Some(cat_smf_editor::chain::default_instrument()?)
        } else {
            None
        };
        automation::resolve(&collection, track, &patches, fallback.as_ref())?;
        owners
            .entry(owner)
            .or_default()
            .push(Arc::new(curve.clone()));
    }
    crate::view::macro_editor::plans(app, parts, &mut owners)?;
    let midi_owners: Vec<_> = parts
        .iter()
        .filter(|(_, _, s)| {
            s.events
                .iter()
                .any(|e| matches!((e.word >> 20) & 15, 11 | 14))
        })
        .map(|(owner, _, _)| *owner)
        .collect();
    let mut plans = owners
        .into_iter()
        .map(|(owner, curves)| {
            let tempo = Arc::new(TempoMap::new(&loaded.music, owner));
            let mut ticks: Vec<_> = loaded.music.tracks[owner]
                .events
                .iter()
                .filter(|e| matches!(e.status & 0xf0, 0x80 | 0x90))
                .map(|e| e.tick)
                .chain(curves.iter().flat_map(|c| c.points.iter().map(|p| p.tick)))
                .collect();
            ticks.sort_unstable();
            ticks.dedup();
            let boundaries = ticks
                .into_iter()
                .map(|tick| (tick, tempo.seconds_at(tick)))
                .collect();
            Arc::new(OwnerPlan {
                windows: Default::default(),
                midi: midi_owners.contains(&owner),
                live: crate::view::macro_editor::live(app, owner),
                owner,
                curves,
                tempo,
                boundaries,
            })
        })
        .collect();
    performance::prepare(app, parts, &collection, &mut plans)?;
    Ok(plans)
}

pub(crate) mod pattern;
mod performance;
mod runtime;
pub use runtime::RuntimePlan;

/// True when this helper owns the audition, including a rejected target.
pub fn audition_notes(
    app: &mut App,
    track: usize,
    tick: u64,
    notes: &[(u8, u8, Option<f64>)],
    take: Option<Arc<crate::play::sung::Take>>,
) -> bool {
    if app
        .patterns
        .editing
        .as_ref()
        .is_some_and(|c| c.standalone && c.track == track)
    {
        return pattern::audition_notes(app, tick, notes, take);
    }
    let Some(loaded) = &app.loaded else {
        return false;
    };
    let collection = match parameter_curve::read(&loaded.music) {
        Ok(collection) => collection,
        Err(error) => {
            app.audition.stop();
            app.load_error = Some(format!("Automation: {error}"));
            return true;
        }
    };
    if cat_smf_editor::score::macro_values::read(&loaded.music.tracks[track])
        .map_or(true, |v| v.parameters.is_empty())
        && !collection.curves.iter().any(|(&index, curve)| {
            !curve.points.is_empty() && collection.owner(index) == Some(track)
        })
    {
        return false;
    }
    let notes: Vec<_> = notes
        .iter()
        .map(
            |&(pitch, velocity, duration)| crate::play::audio_sequence::Note {
                start: 0.0,
                duration: duration.unwrap_or(f64::INFINITY),
                pitch,
                velocity,
            },
        )
        .collect();
    let end = notes.iter().map(|n| n.duration).fold(0.0, f64::max);
    let origin = TempoMap::new(&loaded.music, track).seconds_at(tick);
    let parts = vec![(
        track,
        loaded.patches[track].clone(),
        Sequence::new(notes, end),
    )];
    let automation = Automation::scoped(app, &parts, false, &[track]);
    if let Some(error) = &automation.error {
        app.audition.stop();
        app.load_error = Some(format!("Automation: {error}"));
        return true;
    }
    #[cfg(test)]
    {
        app.audition.auditioned = parts[0].2.notes.iter().map(|n| n.pitch).collect();
    }
    app.audition.sequences_automated(
        parts,
        take.map(|t| vec![(track, t)]).unwrap_or_default(),
        &automation,
        origin,
    );
    true
}

#[cfg(test)]
mod tests;
