//! First-run dependencies, ahead of the recent-files UI and its keyboard handlers.
mod download;
mod scan;
use crate::ffi;
#[path = "../../../cat-plugin-player/shared/plugin_cache.rs"]
mod metadata;

use crate::{app::App, first_instrument_install::FirstInstrument};
use eframe::egui;
use std::{path::PathBuf, sync::mpsc, time::Duration};

#[derive(Default)]
pub struct Setup {
    pub initial: Option<Option<PathBuf>>,
    download: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    scan: Option<scan::Scan>,
    instrument: FirstInstrument,
    error: Option<String>,
    destination: Option<PathBuf>,
    explicit: Option<PathBuf>,
    active: bool,
}

impl Setup {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn begin(&mut self, explicit: Option<PathBuf>, ctx: &egui::Context) {
        self.explicit = explicit.clone();
        self.active = true;
        self.error = None;
        let result = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| {
                match super::selection::resolve(explicit.clone(), &exe, std::env::var_os("PATH")) {
                    Ok(_) if cached_instrument() => {
                        self.active = false;
                        Ok(())
                    }
                    Ok(_) => self.start_scan(false),
                    Err(error) if explicit.is_some() => Err(error),
                    Err(_) => {
                        self.destination =
                            Some(exe.parent().ok_or("No executable directory")?.to_owned());
                        self.download(ctx);
                        Ok(())
                    }
                }
            });
        if let Err(error) = result {
            self.fail(error);
        }
    }

    fn start_scan(&mut self, rescan: bool) -> Result<(), String> {
        self.scan = Some(scan::Scan::start(rescan)?);
        Ok(())
    }

    fn download(&mut self, ctx: &egui::Context) {
        let directory = self.destination.clone().expect("download destination");
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.download = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(download::install(&directory));
            ctx.request_repaint();
        });
    }

    fn fail(&mut self, error: String) {
        crate::store::error_log::write("setup", &error);
        self.error = Some(error);
    }

    fn poll(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        let mut installed = None;
        if let Some(result) = self.download.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Download worker stopped".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        }) {
            self.download = None;
            match result {
                Ok(path) => {
                    installed = Some(path);
                    if let Err(error) = self.start_scan(true) {
                        self.fail(error);
                    }
                }
                Err(error) => self.fail(error),
            }
        }
        if let Some(result) = self.scan.as_ref().and_then(scan::Scan::poll) {
            self.scan = None;
            match result {
                Ok(plugins) => {
                    let surge = self.instrument.on_scan(&plugins);
                    if surge.is_some() {
                        self.instrument.finish_loading();
                    }
                }
                Err(error) => self.fail(error),
            }
        }
        if let Some(result) = self.instrument.show(ctx, self.scan.is_none()) {
            match result {
                Ok(()) => {
                    if let Err(error) = self.start_scan(true) {
                        self.fail(error);
                    }
                }
                Err(error) => crate::store::error_log::write("setup", &error),
            }
        }
        installed
    }
}

fn cached_instrument() -> bool {
    if cfg!(feature = "recording-mode") {
        return false;
    }
    let Some(config) = dirs::config_local_dir().map(|p| p.join("cat-plugin-player/config.toml"))
    else {
        return false;
    };
    metadata::load(&config)
        .ok()
        .flatten()
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.kind == "instrument"
                    && std::path::Path::new(&entry.plugin.bundle_path).exists()
            })
        })
}

pub fn show(app: &mut App, ctx: &egui::Context) -> bool {
    let blocked = show_setup(app, ctx);
    if !blocked && let Some(mid) = app.player_selection.setup.initial.take() {
        app.open_initial(mid);
        if app.config.analysis_open {
            crate::view::analysis::toggle(app);
        }
        crate::play::playback::start(app, crate::play::playback::Mode::AllTracks);
    }
    blocked
}

fn show_setup(app: &mut App, ctx: &egui::Context) -> bool {
    let setup = &mut app.player_selection.setup;
    if !setup.active {
        return false;
    }
    if let Some(path) = setup.poll(ctx) {
        app.player_selection.executable = Some(path);
        app.patch_roots = crate::player::patch_roots::Cache::new(crate::store::config::directory());
    }
    if let Some(error) = setup.error.clone() {
        let mut retry = false;
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("dependency-error")).show(ctx, |ui| {
            ui.heading("Setup");
            ui.colored_label(egui::Color32::LIGHT_RED, error);
            ui.hyperlink_to("Download cat-plugin-player", super::selection::DOWNLOAD);
            ui.horizontal(|ui| {
                retry = ui.button("Retry").clicked();
                close = ui.button("Later").clicked();
            });
        });
        if retry {
            setup.begin(setup.explicit.clone(), ctx);
        }
        if close || modal.should_close() {
            setup.active = false;
        }
        return true;
    }
    if setup.download.is_some() || setup.scan.is_some() {
        egui::Modal::new(egui::Id::new("dependency-progress")).show(ctx, |ui| {
            ui.heading(if setup.download.is_some() {
                "cat-plugin-player"
            } else {
                "Audio plugins"
            });
            ui.spinner();
        });
        ctx.request_repaint_after(Duration::from_millis(50));
        return true;
    }
    if setup.instrument.dialog_open() {
        ctx.request_repaint_after(Duration::from_millis(100));
        return true;
    }
    // A dismissed install still needs polling until its worker finishes.
    if !setup.instrument.running() {
        setup.active = false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_allows_tab_and_blocks_editor_shortcuts() {
        let mut app = App::empty();
        app.player_selection.setup.active = true;
        app.player_selection.setup.error = Some("Offline".into());
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            crate::view::show(&mut app, ctx)
        });
        let mut raw = egui::RawInput::default();
        for key in [egui::Key::Tab, egui::Key::V] {
            raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        eframe::App::raw_input_hook(&mut app, &ctx, &mut raw);
        assert_eq!(raw.events.len(), 2);
        let before = app.note_mode;
        let _ = ctx.run(raw, |ctx| crate::view::show(&mut app, ctx));
        assert_eq!(app.note_mode, before);
        assert!(ctx.memory(|memory| memory.focused()).is_some());
    }
}
