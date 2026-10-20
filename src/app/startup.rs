//! Restore just enough to play before creating the GUI; finish on the UI thread.
use super::App;
use crate::{
    play::{playback, transport},
    store,
};
use eframe::egui;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct State {
    pub pending: bool,
    restore_position: bool,
    initial: Option<PathBuf>,
    started: Option<Instant>,
    read_error: Option<String>,
}

impl App {
    pub fn bootstrap(player: Option<PathBuf>, mid: Option<PathBuf>) -> Self {
        Self::bootstrap_with(Self::empty(), player, mid, store::config::directory())
    }

    fn bootstrap_with(
        mut app: Self,
        player: Option<PathBuf>,
        mid: Option<PathBuf>,
        root: Option<PathBuf>,
    ) -> Self {
        let _span = cat_smf_editor::startup_trace::span("bootstrap");
        app.config.path = root.as_ref().map(|p| p.join("status.json"));
        app.startup = State {
            pending: true,
            restore_position: mid.is_none(),
            initial: mid.clone(),
            started: Some(Instant::now()),
            read_error: None,
        };
        app.player_selection.executable = player;
        let status = if mid.is_some() {
            Ok(store::startup::Status::default())
        } else {
            let _span = cat_smf_editor::startup_trace::span("minimal_status_read");
            root.ok_or("LOCALAPPDATA is unavailable".into())
                .and_then(|root| store::startup::read(&root.join("status.json")))
        };
        match status {
            Ok(status) => app.config.navigation.last_mid = status.last_mid,
            Err(error) => app.startup.read_error = Some(error),
        }
        // An explicit bad player remains an error, even if cached states would suffice.
        let valid_player = app
            .player_selection
            .executable
            .as_ref()
            .is_none_or(|p| p.is_file());
        if valid_player && app.startup.read_error.is_none() {
            app.open_initial(mid);
            if app.loaded.as_ref().is_some_and(|loaded| {
                loaded
                    .music
                    .tracks
                    .iter()
                    .any(cat_smf_editor::voicevox::is_singer)
            }) {
                // The singer may need the configured executable before deferred restore.
                app.config.voicevox_engine = app
                    .config
                    .path
                    .clone()
                    .map(store::config::Store::open)
                    .and_then(|config| config.voicevox_engine);
            }
            if app.loaded.is_some() {
                playback::start(&mut app, playback::Mode::AllTracks);
                transport::advance(&mut app);
                // A started stream can still be waiting for its first callback.
                // Do not compete with it by starting GUI/font/status I/O yet.
                let waiting = Instant::now();
                let singing = crate::play::sung::prerender::running(&app);
                while !app.audition.is_disabled()
                    && !singing
                    && transport::preparing(&app)
                    && waiting.elapsed() < Duration::from_secs(10)
                {
                    app.audition.pump();
                    std::thread::sleep(Duration::from_millis(1));
                }
                if !app.audition.is_disabled() && !singing && transport::preparing(&app) {
                    app.audition.stop();
                    app.playback.active = None;
                    app.playback.pending = None;
                    app.load_error = Some("Audio device did not start playback".into());
                }
            }
        }
        app
    }

    pub fn attach(
        mut self,
        cc: &eframe::CreationContext<'_>,
        settings_error: Option<String>,
    ) -> Self {
        self.settings_error = settings_error;
        if self.loaded.is_none() || self.audition.error().is_some() {
            let mid = self.startup.initial.clone();
            self.finish_startup(&cc.egui_ctx);
            self.player_selection
                .setup
                .begin(self.player_selection.executable.clone(), &cc.egui_ctx);
            if self.player_selection.setup.is_active() {
                self.player_selection.setup.initial = Some(mid);
            } else if self.loaded.is_none() {
                self.open_initial(mid);
                playback::start(&mut self, playback::Mode::AllTracks);
            }
        }
        self
    }

    /// Until this completes, only startup/setup handling may access the document.
    pub(crate) fn startup_pending(&mut self, ctx: &egui::Context) -> bool {
        if !self.startup.pending {
            return false;
        }
        // A singer can take longer than the audio-device deadline. Restore the
        // UI now so its progress is visible and normal polling can finish the job.
        if crate::play::sung::prerender::running(self) {
            self.finish_startup(ctx);
            return false;
        }
        let stopped = ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Space));
        let closing = ctx.input(|i| i.viewport().close_requested());
        if !stopped
            && !closing
            && transport::preparing(self)
            && self.startup.started.unwrap().elapsed() < Duration::from_secs(10)
        {
            self.audition.pump();
            ctx.request_repaint_after(Duration::from_millis(5));
            return true;
        }
        let timed_out = !stopped && !closing && transport::preparing(self);
        self.finish_startup(ctx);
        if stopped || closing || timed_out {
            transport::stop(self);
        }
        if timed_out {
            self.load_error = Some("Audio device did not start playback".into());
        }
        false
    }

    fn finish_startup(&mut self, ctx: &egui::Context) {
        let startup = std::mem::take(&mut self.startup);
        let _span = cat_smf_editor::startup_trace::span("deferred_restore");
        let current = self.loaded.as_ref().map(|l| l.original.clone());
        let status_span = cat_smf_editor::startup_trace::span("status_read");
        self.config = self
            .config
            .path
            .clone()
            .map_or_else(store::config::Store::local, store::config::Store::open);
        self.mascot.enabled = self.config.mascot_enabled;
        drop(status_span);
        if let Some(error) = startup.read_error {
            self.config.error = Some(error);
        }
        let recent_span = cat_smf_editor::startup_trace::span("history_read");
        self.recent.history = self
            .config
            .path
            .as_ref()
            .map_or_else(store::recent::History::local, |p| {
                store::recent::History::open(p.with_file_name("recent-files.json"))
            });
        drop(recent_span);
        let display_span = cat_smf_editor::startup_trace::span("score_display_build");
        if let Some(loaded) = &mut self.loaded {
            for (index, track) in loaded.music.tracks.iter().enumerate() {
                if !crate::player::bulk::has_notes(track) {
                    match super::sound::resolve_track(
                        track,
                        self.player_selection.executable.clone(),
                        &mut self.patch_roots,
                        &mut self.state_cache,
                    ) {
                        Ok((patch, gain)) => {
                            loaded.patches[index] = patch;
                            self.audition.set_gain(index, gain);
                        }
                        Err(error) => self.load_error = Some(error),
                    }
                }
            }
            loaded.refresh_display();
            if let Err(error) = self.saved.finish_loading(&loaded.music) {
                self.load_error = Some(error);
            }
        }
        drop(display_span);
        let session_document = self
            .config
            .session_document
            .as_ref()
            .and_then(|p| std::fs::canonicalize(p).ok());
        if session_document != current {
            self.config.session = None;
        }
        if startup.restore_position
            && let Some(session) = self.config.session.clone()
        {
            self.restore_position(&session);
        }
        let persist_span = cat_smf_editor::startup_trace::span("history_status_write");
        if let Some(path) = current {
            self.config.navigation.last_mid = Some(path);
            let loaded = self.loaded.as_ref().unwrap();
            self.recent.history.record(&loaded.original, &loaded.music);
            self.config.persist();
        }
        drop(persist_span);
        let font_span = cat_smf_editor::startup_trace::span("fonts");
        install_fonts(ctx);
        drop(font_span);
        if self.config.analysis_open {
            crate::view::analysis::toggle(self);
        }
        cat_smf_editor::startup_trace::restored();
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for font in [
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
    ] {
        if let Ok(bytes) = std::fs::read(font) {
            fonts
                .font_data
                .insert("japanese".into(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("japanese".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests;
