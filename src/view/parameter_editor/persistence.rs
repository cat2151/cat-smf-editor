use super::*;
use cat_smf_editor::message;
pub(super) fn current(app: &App) -> Result<Curve, String> {
    if let Some(layer) = app.parameter_editor.variation_layer {
        return variation::current(app, layer);
    }
    if let Some((id, target)) = &app.parameter_editor.pattern_curve {
        let music = &app.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
        let catalog = cat_smf_editor::score::pattern::Catalog::read(music)?;
        let definition = catalog.patterns.get(id).ok_or(message::MISSING_PATTERN)?;
        return cat_smf_editor::score::pattern::automation::curves(music, definition)?
            .into_iter()
            .find(|c| &c.target == target)
            .ok_or("patternのparameterが見つかりません".into());
    }
    parameter_curve::read(&app.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music)?
        .curves
        .remove(&source_track(app))
        .ok_or(message::NOT_A_PARAMETER_TRACK.into())
}
pub(super) fn store(app: &mut App, next: Curve) {
    let result = current(app).and_then(|old| {
        if old == next {
            return Ok(());
        }
        if let Some((id, _)) = app.parameter_editor.pattern_curve.clone() {
            return crate::view::patterns::workspace::mutate(app, |m| {
                cat_smf_editor::score::pattern::automation::write(m, id, next, None)
            });
        }
        let track = source_track(app);
        let variation = app.parameter_editor.variation_layer.is_some();
        app.edit_music(|music| {
            if variation {
                cat_smf_editor::score::macro_curve::write(&mut music.tracks[track], &next)
            } else {
                parameter_curve::write(music, track, &next)
            }
        })
    });
    if let Err(e) = result {
        app.parameter_editor.error = Some(e);
    }
}

pub(super) fn curve_owner(app: &App) -> Result<usize, String> {
    let music = &app.loaded.as_ref().ok_or(message::NO_SMF_LOADED)?.music;
    if let Some((id, _)) = &app.parameter_editor.pattern_curve {
        return cat_smf_editor::score::pattern::Catalog::read(music)?
            .patterns
            .get(id)
            .map(|p| p.track)
            .ok_or(message::MISSING_PATTERN.into());
    }
    parameter_curve::read(music)?
        .owner(source_track(app))
        .ok_or("automationの持ち主が見つかりません".into())
}
