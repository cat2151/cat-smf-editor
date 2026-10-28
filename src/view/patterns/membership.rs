use super::*;
use cat_smf_editor::score::pattern::{automation, operations, Catalog};
pub(crate) fn move_to(a: &mut App, id: u64, container: u64) -> Result<(), String> {
    let loaded = a.loaded.as_ref().ok_or("No SMF is loaded")?;
    let catalog = Catalog::read(&loaded.music)?;
    let p = catalog.patterns.get(&id).ok_or("Missing pattern")?;
    if p.container == container {
        return Ok(());
    }
    let target = *catalog
        .tracks
        .get(&container)
        .filter(|t| catalog.sources.contains(t))
        .ok_or("Select a container")?;
    let patch = loaded
        .patches
        .get(target)
        .ok_or("Missing destination sound")?
        .clone();
    let curves = automation::curves(&loaded.music, p)?;
    let tick = catalog
        .patterns
        .values()
        .filter(|p| p.track == target)
        .map(|p| p.range().map(|r| r.1))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .map_or(Ok(0), |end| end.checked_add(1).ok_or("Storage overflow"))?;
    if !curves.is_empty() {
        crate::play::transport::stop(a);
    }
    let baselines =
        crate::view::parameter_editor::pattern_targets::validate(a, target, &patch, &curves, true)?;
    workspace::mutate(a, |m| {
        for (curve, value) in baselines {
            automation::parameter_baseline(m, container, &curve, value)?;
        }
        operations::relocate(m, id, container, tick)
    })?;
    if a.patterns.selected == Some(id) {
        a.patterns.container = Some(container);
    }
    Ok(())
}
