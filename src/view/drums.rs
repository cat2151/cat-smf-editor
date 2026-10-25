//! Track-local 4 × 16 grid with GM notes and independent instrument rows.
use crate::app::App;
use cat_smf_editor::{
    patch_reference::PatchReference,
    score::{
        drums,
        instrument::{Patch, ResolvedLayer},
    },
};
use eframe::egui::{self, Key};
mod catalog;
#[cfg(test)]
mod catalog_tests;
pub(crate) mod commands;
mod grid;
#[cfg(test)]
mod layout_tests;
#[cfg(test)]
mod native_tests;
pub(crate) mod playback;
#[cfg(test)]
mod tests;
mod widgets;

#[derive(Default)]
pub struct State {
    pub open: bool,
    pub(super) confirm_create: bool,
    row: usize,
    field: usize,
    page: u64,
    help: bool,
    first: u64,
    last: u64,
    all_tracks: bool,
    loop_enabled: bool,
    owned_range: Option<(u64, u64)>,
    pub(super) input: Option<Input>,
    catalog: Option<[Vec<PatchReference>; 4]>,
    error: Option<String>,
}

pub fn add_track(app: &mut App) {
    if let Err(error) = create_track(app) {
        app.load_error = Some(error);
    }
}

fn create_track(app: &mut App) -> Result<(), String> {
    create_track_with(app, prepare)
}

fn create_track_with(
    app: &mut App,
    prepare: impl FnOnce(&mut App) -> Result<(Patch, Catalog), String>,
) -> Result<(), String> {
    let mut music = app.loaded.as_ref().ok_or("No song")?.music.clone();
    drums::tick(music.division, 0)?;
    if music.tracks.len() >= 32767 {
        return Err("Too many SMF tracks".into());
    }
    let (patch, catalog) = prepare(app)?;
    let mut source = cat_smf_editor::model::Track {
        events: vec![cat_smf_editor::model::Event {
            tick: 0,
            delta: 0,
            status: 0xff,
            meta_type: 3,
            data: b"Drums".to_vec(),
        }],
    };
    cat_smf_editor::score::instrument::write(
        &mut source,
        &patch,
        &mut app
            .patch_roots
            .lookup(app.player_selection.executable.clone()),
    )?;
    music.tracks.push(source);
    if music.format == 0 {
        music.format = 1;
    }
    let track = music.tracks.len() - 1;
    crate::play::transport::stop(app);
    app.edit_music(|m| {
        *m = music;
        Ok(())
    })?;
    app.loaded.as_mut().unwrap().patches.push(patch);
    app.select_track(track);
    open(app);
    app.drum_editor.catalog = Some(catalog);
    Ok(())
}
pub(super) enum Input {
    Note { text: String, focus: bool },
}

pub fn open(app: &mut App) {
    if app.loaded.is_none() || super::parameter_editor::is_track(app) {
        return;
    }
    let division = app.loaded.as_ref().unwrap().music.division;
    if let Err(e) = drums::tick(division, 0) {
        app.load_error = Some(e);
        return;
    }
    let tick = crate::play::playback::location(app).0;
    let number = app
        .loaded
        .as_ref()
        .and_then(|l| l.time.map(app.track))
        .map_or(1, |map| {
            u64::try_from(map.position(tick).measure).unwrap_or(1)
        });
    playback::close(app);
    let page = number.saturating_sub(1);
    app.drum_editor = State {
        open: true,
        confirm_create: needs_track(app),
        field: 2,
        page,
        first: number,
        last: number,
        loop_enabled: true,
        ..Default::default()
    };
    app.layer_editor.open = false;
    app.piano_roll.open = false;
    app.tempo_curve.open = false;
    app.parameter_editor.close();
    app.effect_editor = Default::default();
    app.playback.open = false;
    app.playback.mixer_open = false;
    app.comma_menu.open = false;
    app.edit_menu.open = false;
    app.help_open = false;
    app.track_view.go_open = false;
    app.track_view.split_open = false;
    app.note_edit = None;
    app.trill_edit = None;
    if !app.drum_editor.confirm_create {
        let r = ensure(app)
            .and_then(|()| app.edit_music(|_| Ok(())))
            .and_then(|()| playback::restart(app));
        result(app, r);
    }
}

fn needs_track(app: &App) -> bool {
    app.loaded.as_ref().is_some_and(|loaded| {
        loaded.patches[app.track].drums.is_none()
            && (loaded.music.tracks[app.track]
                .events
                .iter()
                .any(cat_smf_editor::score::note_view::note_on)
                || loaded.patches[app.track] != Patch::default())
    })
}

fn ensure(app: &mut App) -> Result<(), String> {
    let track = app.track;
    if app.loaded.as_ref().ok_or("No song")?.patches[track]
        .drums
        .is_some()
    {
        return Ok(());
    }
    if needs_track(app) {
        app.drum_editor.confirm_create = true;
        return Ok(());
    }
    let (patch, catalog) = prepare(app)?;
    crate::player::selection::store_track(app, track, &patch)?;
    app.drum_editor.catalog = Some(catalog);
    Ok(())
}

type Catalog = [Vec<PatchReference>; 4];
fn prepare(app: &mut App) -> Result<(Patch, Catalog), String> {
    let catalog = catalog::load(app)?;
    let settings = drums::Settings::default();
    let mut patch = Patch {
        layers: Vec::new(),
        drums: Some(settings.clone()),
        effects: Vec::new(),
    };
    let explicit = app.player_selection.executable.clone();
    for (i, entries) in catalog.iter().enumerate() {
        let reference = &entries[0];
        let instrument = app.state_cache.resolve(reference, |r| {
            crate::player::state_request::generate(r, explicit.clone())
        })?;
        patch.layers.push(ResolvedLayer {
            settings: cat_smf_editor::instrument_layers::Layer {
                id: settings.rows[i].layer,
                ..Default::default()
            },
            instrument: Some(instrument),
        });
    }
    Ok((patch, catalog))
}

impl State {
    pub(crate) fn command_target(&self) -> (usize, usize, u64) {
        (self.row, self.field, self.page)
    }
}

fn activate(app: &mut App) -> Result<(), String> {
    ensure(app)?;
    if app.drum_editor.confirm_create {
        return Ok(());
    }
    let state = &app.drum_editor;
    let row = state.row;
    let track = app.track;
    let patch = &app.loaded.as_ref().unwrap().patches[track];
    let settings = patch.drums.as_ref().ok_or("No drum settings")?;
    match state.field {
        0 => {
            let layer = settings.rows[row].layer;
            crate::player::selection::open_layer(app, layer);
        }
        1 => {
            app.drum_editor.input = Some(Input::Note {
                text: settings.rows[row].output.to_string(),
                focus: true,
            })
        }
        field => {
            let number = state.page + 1;
            app.edit_music(|m| drums::toggle_measure(m, track, row, number, field - 2))?;
        }
    }
    Ok(())
}

fn confirm(app: &mut App) -> Result<(), String> {
    let row = app.drum_editor.row;
    let mut patch = app.loaded.as_ref().ok_or("No song")?.patches[app.track].clone();
    match app.drum_editor.input.as_ref().ok_or("No input")? {
        Input::Note { text, .. } => {
            let note: u8 = text.parse().map_err(|_| "MIDI note must be 0–127")?;
            if note > 127 {
                return Err("MIDI note must be 0–127".into());
            }
            patch.drums.as_mut().ok_or("No drum settings")?.rows[row].output = note;
        }
    }
    crate::player::selection::store_track(app, app.track, &patch)?;
    app.drum_editor.input = None;
    Ok(())
}

fn result(app: &mut App, result: Result<(), String>) {
    app.drum_editor.error = result.err();
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.drum_editor.open {
        return;
    }
    super::commands::route(app, ctx);
    if !app.drum_editor.open {
        return;
    }
    let mut action: Option<commands::Op> = None;
    widgets::show(app, ctx, &mut action);
    if let Some(key) = action {
        commands::apply(app, key);
    }
}
