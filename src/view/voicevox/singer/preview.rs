//! Selection-driven WAV previews and one-at-a-time background reservations.
use super::*;
use crate::play::sung::prerender;
use cat_smf_editor::voicevox::{cache, score};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};
mod queue;
use queue::Queue;
const RATE: u32 = 48000;

#[derive(Default)]
pub(super) struct State {
    job: Option<JoinHandle<prerender::Outcome>>,
    rendering: Option<Voice>,
    selection: Option<Voice>,
    completed: Option<(Voice, Vec<f32>)>,
    queue: Queue,
    ready: HashSet<Voice>,
    failed: HashSet<Voice>,
    catalog: Vec<Voice>,
    directory: Option<PathBuf>,
    pub(super) playback: Option<Playback>,
}
pub(super) struct Playback {
    _player: Option<crate::play::wav::Player>,
}
impl State {
    pub fn running(&self) -> bool {
        self.job.is_some()
    }
    pub fn cached(&self, voice: &Voice) -> bool {
        self.ready.contains(voice)
    }
    #[cfg(test)]
    pub fn reservations(&self) -> Vec<Voice> {
        self.queue.snapshot()
    }
    #[cfg(test)]
    pub fn waiting(&self) -> bool {
        !self.queue.is_empty()
    }
}

pub(super) fn reset(app: &mut App) {
    let state = &mut app.voicevox.catalog.preview;
    state.queue.clear();
    state.selection = None;
    state.playback = None;
    state.completed = None;
    state.catalog.clear();
    state.failed.clear();
}

pub(super) fn retry(app: &mut App) {
    app.voicevox.catalog.error = None;
    app.voicevox.catalog.preview.failed.clear();
}

fn active(app: &App) -> bool {
    !app.voicevox.wizard
        && app
            .voicevox
            .singer
            .as_ref()
            .is_some_and(|p| p.song == app.song_id && p.generation == app.generation)
}

/// Discover existing files once per catalog/opening; successful renders update the set directly.
pub(super) fn inventory(app: &mut App) {
    let directory = prerender::directory(app);
    let state = &mut app.voicevox.catalog.preview;
    if state.catalog == app.voicevox.catalog.voices && state.directory == directory {
        return;
    }
    state.catalog = app.voicevox.catalog.voices.clone();
    state.directory = directory;
    state.ready.clear();
    if let Some(directory) = &state.directory {
        for voice in &state.catalog {
            let score = score::demo(voice.clone());
            if directory
                .join(format!("{}.wav", cache::key(&score, RATE)))
                .is_file()
            {
                state.ready.insert(voice.clone());
            }
        }
    }
}

fn play(app: &mut App, samples: Vec<f32>) {
    let player = if app.audition.is_disabled() {
        Ok(None)
    } else {
        crate::play::wav::Player::play(RATE, samples).map(Some)
    };
    match player {
        Ok(player) => app.voicevox.catalog.preview.playback = Some(Playback { _player: player }),
        Err(error) => app.voicevox.catalog.error = Some(error),
    }
}

/// Called after keyboard, search and scroll layout, so dispatch always uses the latest priorities.
pub(super) fn update(app: &mut App, visible: &[Voice]) {
    if !active(app) {
        reset(app);
        return;
    }
    inventory(app);
    let selected = app
        .voicevox
        .singer
        .as_ref()
        .map(|p| p.selected)
        .and_then(|row| super::filtered(app).get(row).copied())
        .map(|i| app.voicevox.catalog.voices[i].clone());
    let state = &mut app.voicevox.catalog.preview;
    let changed = state.selection != selected;
    let completed = state.completed.take();
    if changed {
        state.playback = None;
        state.selection = selected.clone();
        if let Some(voice) = &selected
            && state.cached(voice)
            && let Some(directory) = &state.directory
        {
            match cache::load_or_sing(directory, &score::demo(voice.clone()), RATE, |_, _| {
                Err(String::new())
            }) {
                Ok(samples) => play(app, samples),
                Err(_) => {
                    app.voicevox.catalog.preview.ready.remove(voice);
                }
            }
        }
    } else if let Some((voice, samples)) = completed
        && selected.as_ref().is_some_and(|s| s.same_style(&voice))
    {
        play(app, samples);
    }
    let state = &mut app.voicevox.catalog.preview;
    let catalog = &app.voicevox.catalog.voices;
    // Rebuild waiting reservations from current demand, excluding completed/failed/in-flight work.
    state.queue.retain_catalog(catalog);
    for voice in catalog {
        if state.ready.contains(voice)
            || state.failed.contains(voice)
            || state
                .rendering
                .as_ref()
                .is_some_and(|v| v.same_style(voice))
        {
            state.queue.cancel(voice);
        } else {
            state.queue.reserve(voice.clone());
        }
    }
    if let Some(voice) = &selected
        && !state.ready.contains(voice)
        && !state.failed.contains(voice)
        && state
            .rendering
            .as_ref()
            .is_none_or(|v| !v.same_style(voice))
    {
        state.queue.front(voice.clone());
    }
    state.queue.deduplicate();
    state.queue.sort(selected.as_ref(), visible, catalog);
    dispatch(app);
}

/// Finish one render. Playback is deferred until update has processed this frame's input.
pub(super) fn poll(app: &mut App) {
    if !active(app) {
        reset(app);
    }
    if !app
        .voicevox
        .catalog
        .preview
        .job
        .as_ref()
        .is_some_and(|j| j.is_finished())
    {
        return;
    }
    let result = app.voicevox.catalog.preview.job.take().unwrap().join();
    let voice = app.voicevox.catalog.preview.rendering.take().unwrap();
    match result {
        Ok(outcome) => {
            if outcome.engine.is_some() {
                app.voicevox.engine = outcome.engine;
            }
            if let Some((_, result)) = outcome.sung.into_iter().next() {
                match result {
                    Ok(samples) => {
                        let state = &mut app.voicevox.catalog.preview;
                        state.ready.insert(voice.clone());
                        state.queue.cancel(&voice);
                        if active(app) {
                            app.voicevox.catalog.preview.completed = Some((voice, samples));
                        }
                    }
                    Err(error) => {
                        app.voicevox.catalog.preview.failed.insert(voice);
                        if active(app) {
                            app.voicevox.catalog.error = Some(error);
                        }
                    }
                }
            }
            if outcome.no_executable && active(app) {
                super::super::request_executable(app);
            }
        }
        Err(_) => {
            app.voicevox.catalog.preview.failed.insert(voice);
            if active(app) {
                app.voicevox.catalog.error = Some("VOICEVOX: sample thread panicked".into());
            }
        }
    }
}

fn dispatch(app: &mut App) {
    if app.voicevox.catalog.preview.running()
        || app.voicevox.catalog.job.is_some()
        || app.voicevox.catalog.error.is_some()
        || prerender::running(app)
        || app.playback.pending.is_some()
    {
        return;
    }
    let Some(directory) = app.voicevox.catalog.preview.directory.clone() else {
        return;
    };
    let Some(voice) = app.voicevox.catalog.preview.queue.pop() else {
        return;
    };
    let score = score::demo(voice.clone());
    #[cfg(not(test))]
    let port = engine::PORT;
    #[cfg(test)]
    let port = app.voicevox.prerender.target.as_ref().map_or(0, |t| t.0);
    let request = prerender::Request {
        port,
        configured: app.config.voicevox_engine.clone(),
        directory,
        rate: RATE,
        scores: vec![(cache::key(&score, RATE), score)],
        #[cfg(test)]
        sing: app.voicevox.prerender.sing,
    };
    let engine = app.voicevox.engine.take();
    match std::thread::Builder::new()
        .name("voicevox-sample".into())
        .spawn(move || prerender::run(request, engine, Arc::new(AtomicBool::new(false))))
    {
        Ok(job) => {
            let state = &mut app.voicevox.catalog.preview;
            state.job = Some(job);
            state.rendering = Some(voice);
        }
        Err(error) => {
            app.voicevox.catalog.preview.failed.insert(voice);
            app.voicevox.catalog.error = Some(error.to_string());
        }
    }
}

pub(super) fn shut_down(app: &mut App) {
    reset(app);
    if let Some(job) = app.voicevox.catalog.preview.job.take()
        && let Ok(outcome) = job.join()
    {
        drop(outcome.engine);
    }
}
