use super::*;
type Live = BTreeMap<Target, Arc<AtomicU64>>;
pub(crate) fn live(app: &App, owner: usize) -> Live {
    app.macro_editor
        .session
        .as_ref()
        .filter(|s| s.owner == owner && valid(app, s))
        .map(|s| {
            s.rows
                .iter()
                .map(|r| (r.curve.target.clone(), r.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}
pub(crate) fn plans(
    app: &App,
    parts: &[(usize, Patch, Sequence)],
    owners: &mut BTreeMap<usize, Vec<Arc<Curve>>>,
) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    for (owner, patch, _) in parts {
        let mut values = cat_smf_editor::score::macro_curve::expand(&loaded.music.tracks[*owner])?;
        let fixed = macro_values::read(&loaded.music.tracks[*owner])?.parameters;
        for c in fixed {
            if !values.iter().any(|v| same_parameter(v, &c)) {
                values.push(c);
            }
        }
        // A removed layer or a replaced plugin must never receive an old setting.
        values.retain(|c| {
            patch.layers.iter().any(|l| {
                l.settings.id == c.target.layer
                    && l.instrument.as_ref().is_some_and(|p| {
                        p.plugin.format == c.target.plugin_format
                            && p.plugin.id == c.target.plugin_id
                    })
            })
        });
        let curves = owners.entry(*owner).or_default();
        for c in values {
            if !curves.iter().any(|existing| same_parameter(existing, &c)) {
                curves.push(Arc::new(c));
            }
        }
        if let Some(s) = app
            .macro_editor
            .session
            .as_ref()
            .filter(|s| s.owner == *owner && valid(app, s))
        {
            for row in &s.rows {
                curves.retain(|c| !same_parameter(c, &row.curve));
                curves.push(Arc::new(row.saved()));
            }
        }
    }
    owners.retain(|_, curves| !curves.is_empty());
    Ok(())
}
fn same_parameter(a: &Curve, b: &Curve) -> bool {
    a.target.layer == b.target.layer
        && a.target.plugin_format == b.target.plugin_format
        && a.target.plugin_id == b.target.plugin_id
        && a.target.parameter_id == b.target.parameter_id
}
