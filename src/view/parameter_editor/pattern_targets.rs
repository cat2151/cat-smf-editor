//! Revalidate container changes before committing their persisted sound or membership.
use super::*;
use cat_smf_editor::score::{
    automation::{validate_parameter, ParameterSpec},
    pattern::{automation, Catalog},
};
pub(crate) fn validate(
    app: &mut App,
    owner: usize,
    patch: &Patch,
    curves: &[Curve],
    native: bool,
) -> Result<Vec<(Curve, f64)>, String> {
    let mut baselines = vec![];
    for curve in curves {
        let key = plugin(patch, curve.target.layer)?;
        if key.id != curve.target.plugin_id || key.format != curve.target.plugin_format {
            return Err(format!(
                "Container instrument is incompatible with {}",
                curve.name
            ));
        }
        if native {
            let parameters = catalog(app, owner, patch, curve.target.layer)?;
            let p = parameters
                .iter()
                .find(|p| p.stable_id == curve.target.parameter_id)
                .ok_or("Destination parameter is unavailable")?;
            validate_parameter(
                curve,
                ParameterSpec {
                    stable_id: &p.stable_id,
                    minimum: p.minimum,
                    maximum: p.maximum,
                    normalized: p.normalized,
                    eligible: p.eligible,
                },
            )?;
            p.checked_current()?;
            baselines.push((curve.clone(), p.current));
        }
    }
    Ok(baselines)
}
pub(crate) fn sound(
    app: &mut App,
    music: &cat_smf_editor::model::Music,
    owner: usize,
    patch: &Patch,
) -> Result<(), String> {
    let catalog = Catalog::read(music)?;
    let mut targets = std::collections::BTreeMap::new();
    for p in catalog.patterns.values().filter(|p| p.track == owner) {
        for c in automation::curves(music, p)? {
            targets.insert(c.target.clone(), c);
        }
    }
    let curves = targets.into_values().collect::<Vec<_>>();
    validate(app, owner, patch, &curves, true)?;
    Ok(())
}

pub(crate) fn changes(
    app: &mut App,
    before: &cat_smf_editor::model::Music,
    next: &cat_smf_editor::model::Music,
    patches: &[Patch],
) -> Result<(), String> {
    let old = Catalog::read(before)?;
    let new = Catalog::read(next)?;
    let mut prior = std::collections::BTreeSet::new();
    for p in old.patterns.values() {
        for c in automation::curves(before, p)? {
            prior.insert(c.target);
        }
    }
    for &owner in &new.sources {
        let patch = patches.get(owner).ok_or("Missing container patch")?;
        let container = *new.tracks.iter().find(|(_, t)| **t == owner).unwrap().0;
        let old_patch = old
            .tracks
            .get(&container)
            .and_then(|t| app.loaded.as_ref()?.patches.get(*t));
        let changed = old_patch != Some(patch);
        let mut curves = std::collections::BTreeMap::new();
        for p in new.patterns.values().filter(|p| p.track == owner) {
            for c in automation::curves(next, p)? {
                curves.insert(c.target.clone(), c);
            }
        }
        for c in curves.into_values() {
            validate(
                app,
                owner,
                patch,
                std::slice::from_ref(&c),
                changed || !prior.contains(&c.target),
            )?;
        }
    }
    Ok(())
}
