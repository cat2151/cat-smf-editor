//! Interactive song loading advances only after its previous progress was presented.
use crate::{
    app::{sound, App},
    play::audition::opening::Preparation,
    store::saved::Document,
};
use cat_smf_editor::model::Music;
use eframe::egui;
use std::path::PathBuf;

mod overlay;
#[cfg(test)]
pub(crate) mod tests;

#[derive(Default)]
pub struct State {
    job: Option<Job>,
    painted: Option<u64>,
}

struct Job {
    path: PathBuf,
    name: String,
    play: bool,
    title: &'static str,
    rows: Vec<(String, &'static str)>,
    current: Option<usize>,
    phase: Phase,
}

enum Phase {
    Read,
    Master(Document, Music),
    Tracks(Document, Music, sound::Sound, usize),
    Install(Document, Music, sound::Sound),
    Device,
    Audio(Preparation, usize),
    MasterAudio(Preparation),
    Stream(Preparation),
    Start,
    AwaitAudio(std::time::Instant),
}

pub fn pending(app: &App) -> bool {
    app.opening.job.is_some()
}

/// The caller has completed the normal save/discard confirmation.
pub fn request(app: &mut App, path: PathBuf, play: bool) {
    if pending(app) {
        return;
    }
    app.recent.open = false;
    app.opening = State {
        job: Some(Job {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path,
            play,
            title: "Reading file…",
            rows: Vec::new(),
            current: None,
            phase: Phase::Read,
        }),
        painted: None,
    };
}

pub fn request_drop(app: &mut App, path: PathBuf) {
    let path = match std::fs::canonicalize(&path) {
        Ok(path) => path,
        Err(error) => {
            app.load_error = Some(format!("{}: {error}", path.display()));
            return;
        }
    };
    if app.allow_departure(super::files::Action::LoadWithProgress(path.clone())) {
        request(app, path, false);
    }
}

fn frame(ctx: &egui::Context) -> u64 {
    super::modal::frame_token(ctx).1
}

pub fn advance(app: &mut App, ctx: &egui::Context) {
    if !pending(app) {
        return;
    }
    if !app
        .opening
        .painted
        .is_some_and(|painted| painted < frame(ctx))
    {
        ctx.request_repaint();
        return;
    }
    let mut job = app.opening.job.take().unwrap();
    app.opening.painted = None;
    if matches!(job.phase, Phase::Read) {
        // Release the preview's plugins after the first progress frame is visible.
        crate::view::recent::prefetch(app, ctx);
    }
    match job.advance(app) {
        Ok(true) => app.opening.job = Some(job),
        Ok(false) => {}
        Err(error) => app.load_error = Some(error),
    }
    ctx.request_repaint();
}

impl Job {
    fn advance(&mut self, app: &mut App) -> Result<bool, String> {
        let phase = std::mem::replace(&mut self.phase, Phase::Read);
        self.phase = match phase {
            Phase::Read => {
                crate::play::transport::stop(app);
                app.load_error = None;
                app.invalid_smf_name = None;
                let mut document = Document::new(app.saved.directory.clone());
                let music = document.load(&self.path).map_err(|error| {
                    if matches!(error, crate::store::saved::LoadError::InvalidSmf(_)) {
                        app.invalid_smf_name = Some(self.name.clone());
                    }
                    error.to_string()
                })?;
                self.rows = music
                    .tracks
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        (
                            format!("{} · {}", i + 1, t.name().unwrap_or_else(|| "—".into())),
                            "Queued",
                        )
                    })
                    .collect();
                self.title = "Restoring master effects…";
                Phase::Master(document, music)
            }
            Phase::Master(document, music) => {
                let master = sound::resolve_master(
                    &music,
                    app.player_selection.executable.clone(),
                    &mut app.patch_roots,
                    &mut app.state_cache,
                )?;
                self.title = "Restoring sounds…";
                self.current = (!self.rows.is_empty()).then_some(0);
                if let Some(row) = self.rows.first_mut() {
                    row.1 = "Restoring sound…";
                }
                Phase::Tracks(
                    document,
                    music,
                    sound::Sound {
                        patches: Vec::new(),
                        master,
                        volumes: Vec::new(),
                    },
                    0,
                )
            }
            Phase::Tracks(document, music, mut sound, index) => {
                if let Some(track) = music.tracks.get(index) {
                    let (patch, volume) = sound::resolve_track(
                        track,
                        app.player_selection.executable.clone(),
                        &mut app.patch_roots,
                        &mut app.state_cache,
                    )
                    .map_err(|e| format!("Track {}: {e}", index + 1))?;
                    sound.patches.push(patch);
                    sound.volumes.push(volume);
                    self.rows[index].1 = "Sound restored";
                    if let Some(row) = self.rows.get_mut(index + 1) {
                        row.1 = "Restoring sound…";
                    }
                    self.current = (index + 1 < self.rows.len()).then_some(index + 1);
                    Phase::Tracks(document, music, sound, index + 1)
                } else {
                    self.title = "Preparing event list…";
                    Phase::Install(document, music, sound)
                }
            }
            Phase::Install(document, music, sound) => {
                app.install_resolved(self.path.clone(), self.name.clone(), music, document, sound);
                if !self.play {
                    return Ok(false);
                }
                for row in &mut self.rows {
                    row.1 = "Queued";
                }
                self.title = "Opening audio device…";
                Phase::Device
            }
            Phase::Device => {
                let preparation = Preparation::new(&app.audition)?;
                self.title = "Preparing tracks…";
                self.select_audio(app, 0);
                Phase::Audio(preparation, 0)
            }
            Phase::Audio(mut preparation, index) => {
                preparation.pump();
                let loaded = app.loaded.as_ref().unwrap();
                if let Some(track) = loaded.music.tracks.get(index) {
                    if crate::player::bulk::has_notes(track) {
                        preparation.track(index, &loaded.patches[index])?;
                    }
                    self.rows[index].1 = "Ready";
                    self.select_audio(app, index + 1);
                    Phase::Audio(preparation, index + 1)
                } else {
                    self.current = None;
                    if !app.audition.master().effects.is_empty() {
                        self.rows.push(("Master".into(), "Loading effects…"));
                        self.current = Some(self.rows.len() - 1);
                    }
                    self.title = "Preparing master effects…";
                    Phase::MasterAudio(preparation)
                }
            }
            Phase::MasterAudio(mut preparation) => {
                preparation.pump();
                preparation.master(app.audition.master())?;
                if let Some(index) = self.current.take() {
                    self.rows[index].1 = "Ready";
                }
                self.title = "Starting audio…";
                Phase::Stream(preparation)
            }
            Phase::Stream(preparation) => {
                preparation.pump();
                preparation.finish(&mut app.audition)?;
                self.title = "Preparing playback…";
                Phase::Start
            }
            Phase::Start => {
                if let Some(error) = app.audition.error() {
                    return Err(error.to_owned());
                }
                crate::play::playback::start(app, crate::play::playback::Mode::AllTracks);
                self.title = "Starting playback…";
                Phase::AwaitAudio(std::time::Instant::now())
            }
            Phase::AwaitAudio(started) => {
                crate::play::transport::advance(app);
                if app.audition.is_disabled() || !crate::play::transport::preparing(app) {
                    return Ok(false);
                }
                // The singer's WAV may take a while; the device wait starts after it.
                if crate::play::sung::prerender::running(app) {
                    self.title = crate::play::sung::prerender::progress(app)
                        .unwrap_or("Preparing VOICEVOX audio…");
                    Phase::AwaitAudio(std::time::Instant::now())
                } else if started.elapsed() > std::time::Duration::from_secs(10) {
                    crate::play::transport::stop(app);
                    return Err("Audio device did not start playback".into());
                } else {
                    Phase::AwaitAudio(started)
                }
            }
        };
        Ok(true)
    }

    fn select_audio(&mut self, app: &App, index: usize) {
        self.current = (index < self.rows.len()).then_some(index);
        if let Some(row) = self.rows.get_mut(index) {
            row.1 = if crate::player::bulk::has_notes(
                &app.loaded.as_ref().unwrap().music.tracks[index],
            ) {
                "Loading instrument…"
            } else {
                "Preparing track…"
            };
        }
    }
}

/// Background views still paint, but cannot edit, audition or navigate during loading.
pub fn suspend_input(app: &App, ctx: &egui::Context) -> Option<egui::InputState> {
    if !pending(app) {
        return None;
    }
    let saved = ctx.input(Clone::clone);
    ctx.input_mut(|i| {
        i.events.clear();
        i.keys_down.clear();
        i.pointer = Default::default();
        i.modifiers = egui::Modifiers::NONE;
        i.raw.events.clear();
        i.raw.dropped_files.clear();
        i.raw_scroll_delta = egui::Vec2::ZERO;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
    });
    Some(saved)
}

pub fn show(app: &mut App, ctx: &egui::Context, input: Option<egui::InputState>) {
    if let Some(input) = input {
        ctx.input_mut(|i| *i = input);
    }
    if let Some(job) = &app.opening.job {
        overlay::show(job, ctx);
        app.opening.painted = Some(frame(ctx));
        ctx.request_repaint();
    }
}
