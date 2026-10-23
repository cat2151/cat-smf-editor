//! Track-local native parameter selection and scalar curve editing.
use crate::app::App;
use cat_smf_editor::{
    edit::parameter_curve::{self, Curve, Point, Target, ValueSpace},
    host::{parameters::Parameter, PluginKey},
    score::instrument::Patch,
};
use eframe::egui;
mod canvas;
mod picker_ui;
pub use picker_ui::keep_ime;
use picker_ui::picker_ui;
mod commands;
mod curve;
mod history;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod picker_navigation_tests;
pub(crate) mod preview;
#[cfg(test)]
mod tests;
mod variation;
mod window;
pub(crate) use variation::open as open_variation;
#[cfg(test)]
mod variation_tests;
pub(crate) use commands::collect;

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// A view opened from its owner keeps the score on that owner track.
    curve_track: Option<usize>,
    variation_layer: Option<u64>,
    picker: Option<Picker>,
    selected: Option<(super::tempo_curve::Target, usize)>,
    input: Option<Input>,
    draft: Option<Curve>,
    drag: bool,
    viewport: super::tempo_curve::viewport::Viewport,
    repeat_drag: super::timeline_repeat::State,
    pub error: Option<String>,
    labels: Vec<(f64, String)>,
    labels_axis: Option<super::tempo_curve::Axis>,
    parameter: Option<Parameter>,
    owner: Option<usize>,
}
struct Input {
    tick: String,
    value: f64,
    focus: bool,
    reveal: bool,
}
struct Picker {
    history: super::selection_history::History,
    help: bool,
    tick: u64,
    previewed: bool,
    title: String,
    song: u64,
    generation: u64,
    owner: usize,
    layer: u64,
    patch: Patch,
    plugin: PluginKey,
    parameters: Vec<Parameter>,
    query: String,
    search: super::commands::SearchQuery,
    selected: usize,
    focus: bool,
    composing: bool,
    reveal: bool,
}
impl State {
    pub fn picker_open(&self) -> bool {
        self.picker.is_some()
    }
    pub fn input_open(&self) -> bool {
        self.input.is_some()
    }
    pub fn close(&mut self) {
        *self = Self::default();
    }
}
pub fn is_track(app: &App) -> bool {
    app.loaded.as_ref().is_some_and(|l| {
        parameter_curve::read(&l.music).is_ok_and(|c| c.curves.contains_key(&app.track))
    })
}
pub(crate) fn layer_label(patch: &Patch, id: u64) -> String {
    patch
        .layers
        .iter()
        .enumerate()
        .find(|(_, l)| l.settings.id == id)
        .map_or_else(
            || "Missing layer".into(),
            |(i, l)| format!("Layer {} · {}", i + 1, l.label()),
        )
}
fn plugin(patch: &Patch, layer: u64) -> Result<PluginKey, String> {
    let l = patch
        .layers
        .iter()
        .find(|l| l.settings.id == layer)
        .ok_or("Missing layer")?;
    l.instrument
        .as_ref()
        .map(|p| Ok(p.plugin.clone()))
        .unwrap_or_else(cat_smf_editor::chain::default_instrument)
}
fn catalog(
    app: &mut App,
    owner: usize,
    patch: &Patch,
    layer: u64,
) -> Result<Vec<Parameter>, String> {
    let previous = if !app.audition.has_parameter_owner(owner, patch) {
        crate::play::selection_playback::suspend(app)
    } else {
        None
    };
    let result = app.audition.parameter_catalog(owner, patch, layer);
    if let Some(previous) = previous {
        crate::play::selection_playback::restore(app, previous);
    }
    result
}
pub fn choose(app: &mut App, layer: u64) {
    match make_picker(app, layer) {
        Ok(picker) => {
            app.parameter_editor.picker = Some(picker);
            app.parameter_editor.error = None;
            app.layer_editor.open = false;
            preview::start(app);
        }
        Err(e) => app.player_selection.error = Some(e),
    }
}
fn make_picker(app: &mut App, layer: u64) -> Result<Picker, String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if loaded.music.format != 1 {
        return Err("Parameter automation requires SMF format 1".into());
    }
    if is_track(app) || cat_smf_editor::voicevox::is_singer(&loaded.music.tracks[app.track]) {
        return Err("Select an instrument owner track".into());
    }
    let patch = loaded.patches[app.track].clone();
    let plugin = plugin(&patch, layer)?;
    let title = format!(
        "{} / {} / {}",
        loaded.track_titles[app.track],
        layer_label(&patch, layer),
        plugin.name
    );
    let parameters = catalog(app, app.track, &patch, layer)?
        .into_iter()
        .filter(|p| p.eligible)
        .collect::<Vec<_>>();
    if parameters.is_empty() {
        return Err("No continuous automatable parameters".into());
    }
    let history = history::load(app);
    Ok(Picker {
        history,
        tick: crate::play::playback::location(app).0,
        previewed: false,
        help: false,
        title,
        song: app.song_id,
        generation: app.generation,
        owner: app.track,
        layer,
        patch,
        plugin,
        parameters,
        query: String::new(),
        search: Default::default(),
        selected: 0,
        focus: true,
        composing: false,
        reveal: true,
    })
}
fn filtered(p: &mut Picker) -> Vec<usize> {
    p.search.update(&p.query);
    let mut indices: Vec<_> = p
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            p.search
                .matches_text(&format!("{} {}", v.path, v.name))
                .then_some(i)
        })
        .collect();
    indices.sort_by_key(|&i| {
        p.history
            .rank(&p.query, &history::identity(&p.plugin, &p.parameters[i]))
    });
    indices
}
fn create(app: &mut App) -> Result<(), String> {
    preview::stop(app);
    let p = app
        .parameter_editor
        .picker
        .as_mut()
        .ok_or("No parameter selection")?;
    let indices = filtered(p);
    let parameter = p
        .parameters
        .get(*indices.get(p.selected).ok_or("No matching parameter")?)
        .ok_or("Parameter disappeared")?
        .clone();
    let (owner, layer, patch, key, song, generation) = (
        p.owner,
        p.layer,
        p.patch.clone(),
        p.plugin.clone(),
        p.song,
        p.generation,
    );
    if app.song_id != song
        || app.generation != generation
        || app.track != owner
        || app.loaded.as_ref().and_then(|l| l.patches.get(owner)) != Some(&patch)
        || plugin(&patch, layer)? != key
    {
        return Err("Parameter target changed; reopen Automation".into());
    }
    let current = catalog(app, owner, &patch, layer)?
        .into_iter()
        .find(|v| v.stable_id == parameter.stable_id && v.eligible)
        .ok_or("Parameter disappeared")?;
    if current.minimum != parameter.minimum
        || current.maximum != parameter.maximum
        || current.normalized != parameter.normalized
    {
        return Err("Parameter instance changed; reopen Automation".into());
    }
    let p = app.parameter_editor.picker.as_ref().unwrap();
    if !p.previewed && current.binding.with_value(0.0) != parameter.binding.with_value(0.0) {
        return Err("Parameter instance changed; reopen Automation".into());
    }
    parameter.checked_current()?;
    let tick = p.tick;
    let query = p.query.clone();
    let identity = history::identity(&key, &parameter);
    let curve = Curve {
        version: 1,
        target: Target {
            group: 0,
            layer,
            plugin_format: key.format,
            plugin_id: key.id,
            parameter_id: current.stable_id,
        },
        value_space: if current.normalized {
            ValueSpace::Normalized
        } else {
            ValueSpace::Plain
        },
        minimum: current.minimum,
        maximum: current.maximum,
        name: current.name,
        path: current.path,
        points: vec![Point {
            tick,
            value: parameter.current,
        }],
        controls: vec![],
    };
    app.create_parameter_track(owner, curve)?;
    history::record(app, &query, &identity);
    app.parameter_editor.picker = None;
    open(app);
    Ok(())
}
pub fn open(app: &mut App) {
    app.parameter_editor = State {
        open: true,
        ..Default::default()
    };
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.layer_editor.open = false;
    if let Err(e) = curve::resolve(app) {
        app.parameter_editor.error = Some(e);
    }
}
pub(crate) fn open_existing(app: &mut App, track: usize) {
    app.parameter_editor = State {
        open: true,
        curve_track: Some(track),
        ..Default::default()
    };
    if let Err(e) = curve::resolve(app) {
        app.parameter_editor.error = Some(e);
    }
}
pub(crate) fn open_existing_variation(app: &mut App, layer: u64) {
    variation::open_view(app, layer);
}
fn source_track(app: &App) -> usize {
    app.parameter_editor.curve_track.unwrap_or(app.track)
}
fn current(app: &App) -> Result<Curve, String> {
    if let Some(layer) = app.parameter_editor.variation_layer {
        return variation::current(app, layer);
    }
    parameter_curve::read(&app.loaded.as_ref().ok_or("No SMF is loaded")?.music)?
        .curves
        .remove(&source_track(app))
        .ok_or("Not a parameter track".into())
}
fn store(app: &mut App, next: Curve) {
    let result = current(app).and_then(|old| {
        if old == next {
            return Ok(());
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
pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.parameter_editor.picker.is_some() {
        picker_ui(app, ctx);
    } else if app.parameter_editor.open {
        curve::show(app, ctx);
    }
}

pub(crate) fn show_preview(
    app: &mut App,
    ctx: &egui::Context,
    lane: Option<usize>,
    layer: Option<u64>,
) {
    app.parameter_editor.open = true;
    app.parameter_editor.curve_track = lane;
    app.parameter_editor.variation_layer = layer;
    curve::show(app, ctx);
}

pub(super) fn value_label(
    audition: &crate::play::audition::Audition,
    owner: usize,
    layer: u64,
    parameter: &Parameter,
    value: f64,
) -> String {
    let text = audition.parameter_text(owner, layer, parameter.binding.with_value(value));
    match parameter.domain() {
        Ok(domain) => domain.describe(value, text.as_deref().map_err(String::as_str)),
        Err(error) => error,
    }
}

pub(super) fn axis_label(
    audition: &crate::play::audition::Audition,
    owner: usize,
    layer: u64,
    parameter: &Parameter,
    value: f64,
) -> String {
    let plugin = audition
        .parameter_text(owner, layer, parameter.binding.with_value(value))
        .unwrap_or_else(|_| "?".into());
    match parameter.domain().and_then(|d| d.position(d.value(value)?)) {
        Ok(position) => format!("{plugin} · pos {:.3}", position.get()),
        Err(error) => error,
    }
}
