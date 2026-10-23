//! Minimal instrument macro editing, with an isolated draft and a fixed measure loop.
use crate::{
    app::App,
    play::{audio_sequence::Sequence, transport},
};
use cat_smf_editor::{
    edit::parameter_curve::{Curve, Point, Target, ValueSpace},
    host::parameters::domain::{Domain, Position},
    score::{instrument::Patch, macro_values},
};
use eframe::egui;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
mod commands;
mod keyboard;
pub(crate) use commands::collect;
mod playback;
pub(crate) use playback::{live, plans};
#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct State {
    session: Option<Session>,
    help: bool,
    error: Option<String>,
}
struct Session {
    owner: usize,
    layer: u64,
    song: u64,
    generation: u64,
    patch: Patch,
    title: String,
    rows: Vec<Row>,
    selected: usize,
    keyboard: keyboard::State,
    variations: Vec<Vec<Curve>>,
    variation: usize,
    readback_at: Option<std::time::Instant>,
}
struct Row {
    curve: Curve,
    /// The API value the audio callback sends; the editor works in 0-1 positions.
    value: Arc<AtomicU64>,
    bipolar: bool,
    readback: Option<(f64, Result<String, String>)>,
}
impl Row {
    fn domain(&self) -> Domain {
        self.curve.domain().expect("validated macro curve")
    }
    fn get(&self) -> f64 {
        let d = self.domain();
        d.value(f64::from_bits(self.value.load(Ordering::Relaxed)))
            .and_then(|v| d.position(v))
            .map_or(0.0, |p| p.get())
    }
    fn set(&self, position: f64) {
        if let Ok(p) = Position::new(position.clamp(0.0, 1.0)) {
            self.set_value(self.domain().api(p).get());
        }
    }
    fn set_value(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }
    fn saved(&self) -> Curve {
        let mut c = self.curve.clone();
        c.points[0].value = f64::from_bits(self.value.load(Ordering::Relaxed));
        c
    }
}
/// The saved API value, else the patch's own 0-1 position mapped onto the curve's range.
fn initial(curve: &Curve, saved: Option<f64>, position: Option<f64>) -> Option<f64> {
    saved.or_else(|| {
        Some(
            curve
                .domain()
                .ok()?
                .api(Position::new(position?).ok()?)
                .get(),
        )
    })
}
impl State {
    pub fn open(&self) -> bool {
        self.session.is_some()
    }
}

const UNSUPPORTED: &str = "Select Surge XT, Vaporizer2, Six Sines or Floe";

fn begin(app: &mut App, layer: u64) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if super::parameter_editor::is_track(app)
        || cat_smf_editor::voicevox::is_singer(&loaded.music.tracks[app.track])
    {
        return Err("Select an instrument track".into());
    }
    let patch = loaded.patches[app.track].clone();
    let slot = patch
        .layers
        .iter()
        .find(|l| l.settings.id == layer)
        .and_then(|l| l.instrument.as_ref())
        .ok_or("No routed macros")?;
    let instrument =
        cat_smf_editor::instrument_macros::Instrument::of(&slot.plugin.format, &slot.plugin.id)
            .ok_or(UNSUPPORTED)?;
    let macros = instrument
        .routed(&slot.plugin.format, &slot.state)
        .ok_or_else(|| format!("Cannot inspect {} macro routing", instrument.name()))?;
    if macros.is_empty() {
        return Err("This patch has no routed macros".into());
    }
    let key = slot.plugin.clone();
    let map = loaded.time.map(app.track).ok_or("Missing measure map")?;
    let mut position = map.position(crate::play::playback::location(app).0);
    position.beat = 1;
    position.tick_numerator = 0;
    position.tick_denominator = 1;
    let a = map.tick_at(&position).ok_or("Missing measure start")?;
    position.measure += 1;
    let b = map.tick_at(&position).ok_or("Missing measure end")?;
    let saved = macro_values::read(&loaded.music.tracks[app.track])?;
    let title = format!(
        "{} / {} / Macros",
        loaded.track_titles[app.track],
        super::parameter_editor::layer_label(&patch, layer)
    );
    transport::stop(app);
    app.audition.restore_automated_baselines();
    let catalog = app.audition.parameter_catalog(app.track, &patch, layer)?;
    let mut rows = vec![];
    for m in macros {
        let n = m.number;
        let mut matching = catalog.iter().filter(|p| {
            instrument.parameter(n, &p.name)
                && p.eligible
                && (p.minimum, p.maximum) == instrument.range()
        });
        let parameter = matching
            .next()
            .ok_or_else(|| format!("Missing Macro {n} parameter"))?;
        if matching.next().is_some() {
            return Err(format!("Ambiguous Macro {n}"));
        }
        let target = Target {
            group: 1,
            layer,
            plugin_format: key.format.clone(),
            plugin_id: key.id.clone(),
            parameter_id: parameter.stable_id.clone(),
        };
        let stored = saved
            .parameters
            .iter()
            .find(|c| c.target == target)
            .map(|c| c.points[0].value);
        let name = m.name.as_ref().map_or_else(
            || format!("Macro {n}"),
            |name| format!("Macro {n} · {name}"),
        );
        let mut curve = Curve {
            version: 1,
            target,
            minimum: parameter.minimum,
            maximum: parameter.maximum,
            value_space: if parameter.normalized {
                ValueSpace::Normalized
            } else {
                ValueSpace::Plain
            },
            name,
            path: parameter.path.clone(),
            points: vec![Point {
                tick: 0,
                value: 0.0,
            }],
            controls: vec![],
        };
        let value = initial(&curve, stored, m.position)
            .ok_or_else(|| format!("Missing Macro {n} saved position"))?;
        curve.points[0].value = value;
        curve.validate()?;
        rows.push(Row {
            curve,
            value: Arc::new(AtomicU64::new(value.to_bits())),
            bipolar: m.bipolar,
            readback: Some((
                parameter.current,
                app.audition
                    .parameter_text(app.track, layer, parameter.binding.with_value(value)),
            )),
        });
    }
    if rows.is_empty() {
        return Err("This patch has no routed macros".into());
    }
    let (variations, variation) = variations::restore(&saved, layer, &rows);
    app.macro_editor = State {
        session: Some(Session {
            owner: app.track,
            layer,
            song: app.song_id,
            generation: app.generation,
            patch,
            title,
            rows,
            selected: 0,
            keyboard: Default::default(),
            variations,
            variation,
            readback_at: None,
        }),
        ..Default::default()
    };
    app.layer_editor.open = false;
    app.comma_menu.open = false;
    app.help_open = false;
    transport::repeat::start(app, (a, b));
    Ok(())
}
pub(crate) fn open_existing(app: &mut App, layer: u64) {
    if let Err(error) = begin(app, layer) {
        super::layers::open(app);
        app.player_selection.error = Some(error);
    }
}
pub(crate) fn cancel(app: &mut App) {
    finish(app, false);
}
fn finish(app: &mut App, confirm: bool) {
    let Some(s) = &app.macro_editor.session else {
        return;
    };
    let owner = s.owner;
    let prepared = if confirm {
        (|| {
            if !valid(app, s) {
                return Err("Macro target changed; reopen Macro edit".into());
            }
            let mut values = macro_values::read(&app.loaded.as_ref().unwrap().music.tracks[owner])?;
            values.parameters.retain(|c| c.target.layer != s.layer);
            values.parameters.extend(s.rows.iter().map(Row::saved));
            values.variations.retain(|v| v.layer != s.layer);
            values.variations.push(s.saved_variations());
            Ok(Some(values))
        })()
    } else {
        Ok(None)
    };
    let result = prepared.and_then(|values| match values {
        Some(values) => app.edit_music(|m| macro_values::write(&mut m.tracks[owner], &values)),
        None => Ok(()),
    });
    if let Err(error) = result {
        app.macro_editor.error = Some(error);
        return;
    }
    transport::stop(app);
    app.audition.restore_automated_baselines();
    app.macro_editor = State::default();
}
fn valid(app: &App, s: &Session) -> bool {
    app.song_id == s.song
        && app.generation == s.generation
        && app.track == s.owner
        && app.loaded.as_ref().and_then(|l| l.patches.get(s.owner)) == Some(&s.patch)
}
pub(crate) fn validate(app: &mut App) {
    if app
        .macro_editor
        .session
        .as_ref()
        .is_some_and(|s| !valid(app, s))
    {
        finish(app, false);
    }
}
mod ui;
pub use ui::show;

mod variations;

mod preview;
pub(crate) use preview::show as show_preview;
