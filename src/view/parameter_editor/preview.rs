//! Transient picker sweep; never writes a curve or changes the patch baseline.
use super::*;
use crate::play::{automation::OwnerPlan, transport};
use cat_smf_editor::score::audition_timing::TempoMap;
use std::sync::Arc;

pub(crate) struct Preview {
    range: (u64, u64),
    song: u64,
    generation: u64,
    owner: usize,
    patch: Patch,
    curve: Arc<Curve>,
}

pub(super) fn stop(app: &mut App) {
    if app.playback.parameter_preview.is_some() {
        transport::stop(app);
    }
}

pub(crate) fn validate(app: &mut App) {
    if app.playback.parameter_preview.as_ref().is_some_and(|p| {
        app.parameter_editor.picker.is_none()
            || (app.playback.pending.is_none() && app.playback.active.is_none())
            || app.song_id != p.song
            || app.generation != p.generation
            || app.track != p.owner
            || app.loaded.as_ref().and_then(|l| l.patches.get(p.owner)) != Some(&p.patch)
    }) {
        stop(app);
    }
}

pub(super) fn start(app: &mut App) {
    stop(app);
    let result = prepare(app);
    match result {
        Ok(Some(preview)) => {
            transport::stop(app);
            let range = preview.range;
            app.playback.parameter_preview = Some(preview);
            app.parameter_editor.picker.as_mut().unwrap().previewed = true;
            transport::repeat::start(app, range);
            if app.playback.pending.is_none() {
                stop(app);
            }
        }
        Ok(None) => {}
        Err(error) => app.parameter_editor.error = Some(error),
    }
}

fn prepare(app: &mut App) -> Result<Option<Preview>, String> {
    let Some(p) = app.parameter_editor.picker.as_mut() else {
        return Ok(None);
    };
    let indices = filtered(p);
    let Some(&index) = indices.get(p.selected) else {
        return Ok(None);
    };
    let parameter = &p.parameters[index];
    parameter.checked_current()?;
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if app.song_id != p.song
        || app.generation != p.generation
        || app.track != p.owner
        || loaded.patches.get(p.owner) != Some(&p.patch)
    {
        return Err("Parameter target changed; reopen Automation".into());
    }
    let map = loaded.time.map(p.owner).ok_or("Missing measure map")?;
    let mut position = map.position(p.tick);
    position.beat = 1;
    position.tick_numerator = 0;
    position.tick_denominator = 1;
    let a = map.tick_at(&position).ok_or("Missing measure start")?;
    position.measure += 1;
    let b = map.tick_at(&position).ok_or("Missing measure end")?;
    let curve = Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: p.layer,
            plugin_format: p.plugin.format.clone(),
            plugin_id: p.plugin.id.clone(),
            parameter_id: parameter.stable_id.clone(),
        },
        value_space: if parameter.normalized {
            ValueSpace::Normalized
        } else {
            ValueSpace::Plain
        },
        minimum: parameter.minimum,
        maximum: parameter.maximum,
        name: parameter.name.clone(),
        path: parameter.path.clone(),
        points: vec![
            Point {
                tick: a,
                value: parameter.minimum,
            },
            Point {
                tick: b,
                value: parameter.maximum,
            },
        ],
        controls: vec![Point {
            tick: a + (b - a) / 2,
            value: parameter.minimum + (parameter.maximum - parameter.minimum) / 2.0,
        }],
    };
    Ok(Some(Preview {
        range: (a, b),
        song: p.song,
        generation: p.generation,
        owner: p.owner,
        patch: p.patch.clone(),
        curve: Arc::new(curve),
    }))
}

/// Replace only the selected lane, retaining the other lanes and accompaniment.
pub(crate) fn apply(app: &App, prepared: &mut transport::Prepared) {
    let Some(preview) = &app.playback.parameter_preview else {
        return;
    };
    let mut owners = prepared.active.automation.owners.as_ref().clone();
    let previous = owners
        .iter()
        .position(|o| o.owner == preview.owner)
        .map(|i| owners.remove(i));
    let mut curves = previous
        .as_ref()
        .map_or_else(Vec::new, |o| o.curves.clone());
    curves.retain(|c| {
        c.target.layer != preview.curve.target.layer
            || c.target.parameter_id != preview.curve.target.parameter_id
    });
    curves.push(preview.curve.clone());
    let tempo = Arc::new(TempoMap::new(
        &app.loaded.as_ref().unwrap().music,
        preview.owner,
    ));
    let mut boundaries = previous
        .as_ref()
        .map_or_else(Vec::new, |o| o.boundaries.clone());
    boundaries.extend(
        preview
            .curve
            .points
            .iter()
            .map(|p| (p.tick, tempo.seconds_at(p.tick))),
    );
    boundaries.sort_by_key(|p| p.0);
    boundaries.dedup_by_key(|p| p.0);
    owners.push(Arc::new(OwnerPlan {
        live: Default::default(),
        owner: preview.owner,
        curves,
        tempo,
        boundaries,
    }));
    prepared.active.automation.owners = Arc::new(owners);
}

#[cfg(test)]
mod local_tests;
#[cfg(test)]
mod tests;
