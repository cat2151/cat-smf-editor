//! Saved macro data rendered by the normal editor, without an audio session.
use super::*;

pub(crate) fn show(app: &mut App, ctx: &egui::Context, layer: u64) {
    let saved = std::mem::take(&mut app.macro_editor);
    match session(app, layer) {
        Ok(session) => {
            app.macro_editor.session = Some(session);
            ui::show(app, ctx);
        }
        Err(error) => {
            crate::view::palette::preview_window(ctx, "Macros").show(ctx, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, error);
            });
        }
    }
    app.macro_editor = saved;
}

fn session(app: &App, layer: u64) -> Result<Session, String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    let patch = loaded.patches[app.track].clone();
    let slot = patch
        .layers
        .iter()
        .find(|l| l.settings.id == layer)
        .and_then(|l| l.instrument.as_ref())
        .ok_or("No instrument")?;
    let instrument =
        cat_smf_editor::instrument_macros::Instrument::of(&slot.plugin.format, &slot.plugin.id)
            .ok_or(UNSUPPORTED)?;
    let macros = instrument
        .routed(&slot.plugin.format, &slot.state)
        .ok_or_else(|| format!("Cannot inspect {} macro routing", instrument.name()))?;
    let saved = macro_values::read(&loaded.music.tracks[app.track])?;
    let mut rows = vec![];
    for m in macros {
        let prefix = format!("Macro {}", m.number);
        let existing = saved.parameters.iter().find(|c| {
            c.target.layer == layer
                && c.target.plugin_format == slot.plugin.format
                && c.target.plugin_id == slot.plugin.id
                && (c.name == prefix || c.name.starts_with(&format!("{prefix} · ")))
        });
        // No host parameter is fabricated or queried. Only the saved display data is
        // used; the temporary curve never leaves this inert rendering session.
        let mut curve = existing.cloned().unwrap_or_else(|| Curve {
            version: 1,
            target: Target {
                group: 1,
                layer,
                plugin_format: slot.plugin.format.clone(),
                plugin_id: slot.plugin.id.clone(),
                parameter_id: String::new(),
            },
            value_space: if slot.plugin.format == "VST3" {
                ValueSpace::Normalized
            } else {
                ValueSpace::Plain
            },
            minimum: instrument.range().0,
            maximum: instrument.range().1,
            name: m
                .name
                .as_ref()
                .map_or_else(|| prefix.clone(), |n| format!("{prefix} · {n}")),
            path: String::new(),
            points: vec![Point {
                tick: 0,
                value: 0.0,
            }],
            controls: vec![],
        });
        let value = initial(&curve, existing.map(|c| c.points[0].value), m.position)
            .ok_or_else(|| format!("Missing {prefix} saved position"))?;
        curve.points[0].value = value;
        curve.name = m
            .name
            .as_ref()
            .map_or_else(|| prefix.clone(), |name| format!("{prefix} · {name}"));
        rows.push(Row {
            curve,
            value: Arc::new(AtomicU64::new(value.to_bits())),
            bipolar: m.bipolar,
            readback: None,
        });
    }
    let (variations, variation) = variations::restore(&saved, layer, &rows);
    Ok(Session {
        owner: app.track,
        layer,
        song: app.song_id,
        generation: app.generation,
        title: format!(
            "{} / {} / Macros",
            loaded.track_titles[app.track],
            crate::view::parameter_editor::layer_label(&patch, layer)
        ),
        patch,
        rows,
        selected: 0,
        keyboard: Default::default(),
        variations,
        variation,
        readback_at: None,
    })
}
