use super::*;
use cat_smf_editor::{
    edit::{parameter_curve, tempo_curve},
    score::{automation, macro_curve, macro_values, note_view::note_on},
};

pub(super) fn entries(app: &App, level: Level) -> Vec<Entry> {
    let Some(loaded) = &app.loaded else {
        return vec![];
    };
    let collection = parameter_curve::read(&loaded.music).unwrap_or_default();
    let default_plugin = cat_smf_editor::chain::default_instrument().ok();
    list(app, level, &collection, default_plugin.as_ref())
}

fn list(
    app: &App,
    level: Level,
    collection: &parameter_curve::Collection,
    default_plugin: Option<&cat_smf_editor::host::PluginKey>,
) -> Vec<Entry> {
    let loaded = app.loaded.as_ref().unwrap();
    let valid_parameter = |track: usize| {
        collection
            .curves
            .get(&track)
            .is_some_and(|c| !c.points.is_empty())
            && automation::resolve_in(
                collection,
                track,
                loaded.patches.iter().enumerate(),
                default_plugin,
            )
            .is_ok()
    };
    let mut result = vec![];
    let mut view = |label: &str, v| result.push(Entry::new(label, Target::View(v)));
    match level {
        Level::Tracks => {
            view("Mixer", View::Mixer);
            view("Dual event list", View::DoubleScore);
            if loaded.music.format == 1
                && (tempo_curve::read(&loaded.music).is_ok_and(|c| c.is_some())
                    || loaded.music.tracks.first().is_some_and(|t| {
                        t.events.iter().any(|e| {
                            e.status == 0xff
                                && e.meta_type == 0x51
                                && e.data.len() == 3
                                && e.data.iter().any(|b| *b != 0)
                        })
                    }))
            {
                view("Tempo", View::Tempo);
            }
            for (track, title) in loaded.track_titles.iter().enumerate() {
                // Dedicated automation lanes are reached through their owning layer.
                if !valid_parameter(track) {
                    result.push(Entry::new(
                        title.clone(),
                        Target::Level(Level::Track(track)),
                    ));
                }
            }
        }
        Level::Track(track) => {
            let Some(source) = loaded.music.tracks.get(track) else {
                return result;
            };
            view("Event list", View::Score(track));
            if source.events.iter().any(note_on) {
                view("Piano roll", View::Piano(track));
            }
            if !collection.curves.contains_key(&track)
                && !cat_smf_editor::voicevox::is_singer(source)
            {
                let patch = &loaded.patches[track];
                if !patch.layers.is_empty() {
                    view("Instrument layers", View::Instruments(track));
                }
                if !patch.effects.is_empty() {
                    view("Effect layers", View::Effects(track));
                }
                for layer in &patch.layers {
                    let level = Level::Layer(track, layer.settings.id);
                    if !list(app, level, collection, default_plugin).is_empty() {
                        result.push(Entry::new(
                            crate::view::parameter_editor::layer_label(patch, layer.settings.id),
                            Target::Level(level),
                        ));
                    }
                }
            }
        }
        Level::Layer(track, layer) => {
            let Some(source) = loaded.music.tracks.get(track) else {
                return result;
            };
            let Some(slot) = loaded.patches[track]
                .layers
                .iter()
                .find(|l| l.settings.id == layer)
                .and_then(|l| l.instrument.as_ref())
            else {
                return parameter_entries(collection, track, layer, &valid_parameter);
            };
            let values = macro_values::read(source).ok();
            let macros =
                cat_smf_editor::surge_macros::supports(&slot.plugin.format, &slot.plugin.id)
                    && cat_smf_editor::surge_macros::parse(
                        &slot.state,
                        slot.plugin.format == "VST3",
                    )
                    .is_some_and(|c| c.routed.iter().any(|v| *v));
            if macros {
                view("Macros", View::Macros(track, layer));
            }
            if macro_curve::expand(source).is_ok()
                && macro_curve::read(source).is_ok_and(|curves| {
                    curves.iter().any(|c| {
                        c.target.layer == layer
                            && !c.points.is_empty()
                            && c.target.plugin_id
                                == format!("{}:{}", slot.plugin.format, slot.plugin.id)
                    })
                })
                && values.as_ref().is_some_and(|v| {
                    v.variations
                        .iter()
                        .any(|v| v.layer == layer && v.snapshots.len() >= 2)
                })
            {
                view("Macro variation curve", View::Variation(track, layer));
            }
            result.extend(parameter_entries(
                collection,
                track,
                layer,
                &valid_parameter,
            ));
        }
    }
    result
}

fn parameter_entries(
    collection: &parameter_curve::Collection,
    owner: usize,
    layer: u64,
    valid: &impl Fn(usize) -> bool,
) -> Vec<Entry> {
    collection
        .curves
        .iter()
        .filter(|&(&track, curve)| {
            collection.owner(track) == Some(owner) && curve.target.layer == layer && valid(track)
        })
        .map(|(&track, curve)| {
            Entry::new(
                format!("Parameter · {} / {}", curve.path, curve.name),
                Target::View(View::Parameter(track)),
            )
        })
        .collect()
}
