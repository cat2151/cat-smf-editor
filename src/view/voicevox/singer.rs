//! Track-scoped voice picker and asynchronous engine catalog.
use crate::{
    app::App,
    view::commands::{Command, Screen},
};
use cat_smf_editor::voicevox::{
    engine::{self, Engine, Failure},
    Voice,
};
use eframe::egui::{self, Key};
use std::thread::JoinHandle;

mod history;
mod keyboard;
pub use keyboard::{collect, keep_ime};
mod preview;

type CatalogResult = (Option<Engine>, Result<Vec<Voice>, Failure>);

#[derive(Default)]
pub struct Catalog {
    pub voices: Vec<Voice>,
    preview: preview::State,
    #[cfg(test)]
    pub fixture: Option<Vec<Voice>>,
    job: Option<JoinHandle<CatalogResult>>,
    pub error: Option<String>,
}

pub struct Picker {
    song: u64,
    generation: u64,
    track: usize,
    pub query: String,
    search: crate::view::commands::SearchQuery,
    selected: usize,
    reveal: bool,
    composing: bool,
    pub focus: bool,
}

fn close(app: &mut App) {
    app.voicevox.singer = None;
    preview::reset(app);
}

pub fn open(app: &mut App) {
    if app.loaded.is_none() {
        return;
    }
    history::load(app);
    preview::reset(app);
    app.voicevox.singer = Some(Picker {
        song: app.song_id,
        generation: app.generation,
        track: app.track,
        query: String::new(),
        search: Default::default(),
        selected: 0,
        reveal: true,
        composing: false,
        focus: true,
    });
    app.voicevox.catalog.error = None;
    app.voicevox.catalog.voices.clear();
    #[cfg(test)]
    if let Some(voices) = &app.voicevox.catalog.fixture {
        app.voicevox.catalog.voices = voices.clone();
    }
}

pub fn poll(app: &mut App, ctx: &egui::Context) {
    preview::poll(app);
    if app
        .voicevox
        .catalog
        .job
        .as_ref()
        .is_some_and(|j| j.is_finished())
    {
        let result = app.voicevox.catalog.job.take().unwrap().join();
        match result {
            Ok((engine, result)) => {
                if engine.is_some() {
                    app.voicevox.engine = engine;
                }
                match result {
                    Ok(voices) => {
                        if voices.is_empty() {
                            app.voicevox.catalog.error = Some("VOICEVOX: no humming voices".into());
                        }
                        if let Some(picker) = &mut app.voicevox.singer {
                            picker.reveal = true;
                            picker.selected = 0;
                        }
                        app.voicevox.catalog.voices = voices;
                    }
                    Err(Failure::NoExecutable) => {
                        app.voicevox.catalog.error = Some("VOICEVOX engine is not set".into());
                        if app.voicevox.singer.is_some() {
                            super::open(app);
                        }
                    }
                    Err(Failure::Failed(error)) => app.voicevox.catalog.error = Some(error),
                }
            }
            Err(_) => app.voicevox.catalog.error = Some("VOICEVOX: catalog thread panicked".into()),
        }
    }
    let catalog = &app.voicevox.catalog;
    if app.voicevox.singer.is_some()
        && catalog.job.is_none()
        && !catalog.preview.running()
        && catalog.voices.is_empty()
        && catalog.error.is_none()
        && !crate::play::sung::prerender::running(app)
    {
        let existing = app.voicevox.engine.take();
        let configured = app.config.voicevox_engine.clone();
        #[cfg(not(test))]
        let port = engine::PORT;
        #[cfg(test)]
        let port = app.voicevox.prerender.target.as_ref().map_or(0, |t| t.0);
        match std::thread::Builder::new()
            .name("voicevox-catalog".into())
            .spawn(move || {
                let engine = match existing {
                    Some(e) if e.client.version().is_ok() => Ok(e),
                    _ => engine::connect(port, configured.as_deref(), engine::START_TIMEOUT),
                };
                match engine {
                    Ok(e) => {
                        let voices = e.client.voices().map_err(Failure::Failed);
                        (Some(e), voices)
                    }
                    Err(error) => (None, Err(error)),
                }
            }) {
            Ok(job) => app.voicevox.catalog.job = Some(job),
            Err(error) => app.voicevox.catalog.error = Some(error.to_string()),
        }
    }
    if running(app) || app.voicevox.singer.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn filtered(app: &mut App) -> Vec<usize> {
    let Some(picker) = &mut app.voicevox.singer else {
        return Vec::new();
    };
    picker.search.update(&picker.query);
    let mut indices: Vec<_> = app
        .voicevox
        .catalog
        .voices
        .iter()
        .enumerate()
        .filter_map(|(i, v)| picker.search.matches_text(&v.label()).then_some(i))
        .collect();
    indices.sort_by_key(|&i| {
        app.selection_history.singer.rank(
            &picker.query,
            &history::identity(&app.voicevox.catalog.voices[i]),
        )
    });
    indices
}

fn confirm(app: &mut App, _: &egui::Context) {
    let Some(picker) = &app.voicevox.singer else {
        return;
    };
    let selected = picker.selected;
    let Some(&index) = filtered(app).get(selected) else {
        return;
    };
    let voice = app.voicevox.catalog.voices[index].clone();
    apply(app, Some(voice));
}

fn apply(app: &mut App, voice: Option<Voice>) {
    let Some(picker) = &app.voicevox.singer else {
        return;
    };
    if picker.song != app.song_id || picker.generation != app.generation {
        close(app);
        return;
    }
    let track = picker.track;
    let current = app
        .loaded
        .as_ref()
        .and_then(|l| l.music.tracks.get(track))
        .and_then(Voice::read);
    if current == voice {
        history::record(app, voice.as_ref());
        close(app);
        return;
    }
    match app.edit_music(|m| {
        Voice::write(
            m.tracks.get_mut(track).ok_or("No such track")?,
            voice.as_ref(),
        )
    }) {
        Ok(()) => {
            history::record(app, voice.as_ref());
            close(app);
            app.voicevox.error = None;
            crate::play::sung::prerender::ahead(app, track);
        }
        Err(error) => app.voicevox.catalog.error = Some(error),
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.voicevox.singer.is_none() || app.voicevox.wizard {
        return;
    }
    crate::view::commands::route(app, ctx);
    let Some(p) = &app.voicevox.singer else {
        preview::reset(app);
        return;
    };
    if p.song != app.song_id || p.generation != app.generation {
        close(app);
        return;
    }
    let mut chosen = None;
    let mut cancel = false;
    let mut disable = false;
    let mut visible = Vec::new();
    let current = app
        .loaded
        .as_ref()
        .and_then(|l| l.music.tracks.get(p.track))
        .and_then(Voice::read);
    preview::inventory(app);
    egui::Modal::new(egui::Id::new("voicevox-singer")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading("VOICEVOX 歌声");
        let p = app.voicevox.singer.as_mut().unwrap();
        let response = ui.add(
            egui::TextEdit::singleline(&mut p.query)
                .id(egui::Id::new("voicevox-singer-query"))
                .desired_width(f32::INFINITY),
        );
        if p.focus {
            response.request_focus();
            p.focus = false;
        }
        if response.changed() {
            p.selected = 0;
            p.reveal = true;
        }
        if app.voicevox.catalog.job.is_some() {
            ui.spinner();
        }
        if let Some(error) = &app.voicevox.catalog.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if app.voicevox.catalog.error.is_some() && ui.button("再取得").clicked() {
            preview::retry(app);
        }
        if let Some(voice) = &current
            && app.voicevox.catalog.job.is_none()
            && !app
                .voicevox
                .catalog
                .voices
                .iter()
                .any(|v| v.same_style(voice))
        {
            ui.colored_label(egui::Color32::LIGHT_RED, voice.label());
        }
        let indices = filtered(app);
        let p = app.voicevox.singer.as_mut().unwrap();
        p.selected = p.selected.min(indices.len().saturating_sub(1));
        crate::view::list_navigation::position(ui, p.selected, indices.len());
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height((ctx.screen_rect().height() * 0.55).max(120.0))
            .show(ui, |ui| {
                for (row, &index) in indices.iter().enumerate() {
                    let voice = &app.voicevox.catalog.voices[index];
                    let label = if app.voicevox.catalog.error.is_none()
                        && !app.voicevox.catalog.preview.cached(voice)
                    {
                        format!("{}  please wait", voice.label())
                    } else {
                        voice.label()
                    };
                    let text = egui::RichText::new(label);
                    let text = if current.as_ref().is_some_and(|v| v.same_style(voice)) {
                        text.color(egui::Color32::LIGHT_GREEN)
                    } else {
                        text
                    };
                    let response = ui.selectable_label(row == p.selected, text);
                    if response.rect.intersects(ui.clip_rect()) {
                        visible.push(voice.clone());
                    }
                    if row == p.selected && p.reveal {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() {
                        p.selected = row;
                        chosen = Some(voice.clone());
                    }
                }
            });
        p.reveal = false;
        ui.horizontal(|ui| {
            if app.voicevox.catalog.preview.running() {
                ui.spinner();
            }
            if ui
                .add_enabled(!indices.is_empty(), egui::Button::new("Enter: 確定"))
                .clicked()
            {
                chosen = indices
                    .get(p.selected)
                    .map(|&i| app.voicevox.catalog.voices[i].clone());
            }
            cancel = ui.button("Esc: 取消").clicked();
            if current.is_some() {
                disable = ui.button("歌声を解除").clicked();
            }
        });
    });
    if chosen.is_none() && !disable && !cancel {
        preview::update(app, &visible);
    }
    if let Some(voice) = chosen {
        apply(app, Some(voice));
    } else if disable {
        apply(app, None);
    } else if cancel {
        close(app);
    }
    if app.voicevox.singer.is_none() {
        preview::reset(app);
    }
}

pub fn shut_down(app: &mut App) {
    preview::shut_down(app);
    if let Some(job) = app.voicevox.catalog.job.take() {
        let _ = job.join();
    }
}

pub fn running(app: &App) -> bool {
    app.voicevox.catalog.job.is_some() || app.voicevox.catalog.preview.running()
}

#[cfg(test)]
mod picker_tests;
