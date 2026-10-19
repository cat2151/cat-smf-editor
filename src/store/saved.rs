//! One MIDI document, saved by confirmation or persisted automatic-save opt-in.
use cat_smf_editor::{model::Music, smf_io};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
pub enum LoadError {
    InvalidSmf(String),
    Other(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSmf(error) | Self::Other(error) => f.write_str(error),
        }
    }
}

#[derive(Clone, Default)]
pub struct Document {
    pub directory: Option<PathBuf>,
    key: Option<PathBuf>,
    untitled: bool,
    baseline: Vec<u8>,
    expected: Vec<u8>,
    bytes: Vec<u8>,
    saved_auto_save: bool,
    auto_save: bool,
    dirty_since: Option<Instant>,
    auto_save_stopped: bool,
    loading: bool,
    pub error: Option<String>,
}

impl Document {
    pub fn new(directory: Option<PathBuf>) -> Self {
        Self {
            directory,
            ..Default::default()
        }
    }

    pub fn load(&mut self, path: &Path) -> Result<Music, LoadError> {
        self.load_for_startup(path, false)
    }

    pub(crate) fn load_for_startup(
        &mut self,
        path: &Path,
        deferred: bool,
    ) -> Result<Music, LoadError> {
        let _span = cat_smf_editor::startup_trace::span(if deferred {
            "document_read_parse"
        } else {
            "document_read_parse_encode"
        });
        let expected = std::fs::read(path)
            .map_err(|e| LoadError::Other(format!("{}: {e}", path.display())))?;
        let music = cat_smf_editor::ffi::parse(&expected).map_err(LoadError::InvalidSmf)?;
        self.key = Some(path.to_owned());
        self.untitled = false;
        self.auto_save = super::auto_save::enabled(&music);
        self.saved_auto_save = self.auto_save;
        self.dirty_since = None;
        self.auto_save_stopped = false;
        self.error = None;
        self.expected = expected;
        self.loading = true;
        if !deferred {
            self.finish_loading(&music).map_err(LoadError::Other)?;
        }
        Ok(music)
    }

    pub(crate) fn finish_loading(&mut self, music: &Music) -> Result<(), String> {
        if self.loading {
            let _span = cat_smf_editor::startup_trace::span("save_baseline_encode");
            let bytes = smf_io::encode(music)?;
            self.baseline = bytes.clone();
            self.bytes = bytes;
            self.loading = false;
        }
        Ok(())
    }

    pub(crate) fn untitled(directory: Option<PathBuf>, music: &Music) -> Result<Self, String> {
        let mut document = Self::new(directory);
        document.untitled = true;
        document.bytes = smf_io::encode(music)?;
        document.dirty_since = Some(Instant::now());
        Ok(document)
    }

    pub(crate) fn is_untitled(&self) -> bool {
        self.untitled
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.key.as_deref()
    }

    pub(crate) fn save_confirmed(&mut self) -> Result<(), String> {
        if !self.is_untitled() {
            return self.save();
        }
        let path = rfd::FileDialog::new()
            .add_filter("SMF", &["mid", "midi", "smf"])
            .set_file_name("untitled.mid")
            .save_file()
            .ok_or("Save cancelled")?;
        self.save_as(&path)
    }

    pub(crate) fn save_as(&mut self, path: &Path) -> Result<(), String> {
        if path.exists() {
            let expected = std::fs::read(path).map_err(|e| e.to_string())?;
            let directory = self
                .directory
                .as_ref()
                .ok_or("Backup directory is unavailable")?;
            super::saved_write::replace_with_backup(
                path,
                &self.bytes,
                &expected,
                &directory.join("backup"),
            )
            .map_err(|e| e.to_string())?;
        } else {
            use std::io::Write;
            let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))
                .map_err(|e| e.to_string())?;
            file.write_all(&self.bytes).map_err(|e| e.to_string())?;
            file.as_file().sync_all().map_err(|e| e.to_string())?;
            file.persist_noclobber(path).map_err(|e| e.to_string())?;
        }
        self.key = Some(path.to_owned());
        self.untitled = false;
        self.expected = self.bytes.clone();
        self.baseline = self.bytes.clone();
        self.saved_auto_save = self.auto_save;
        self.dirty_since = None;
        self.auto_save_stopped = false;
        self.error = None;
        Ok(())
    }

    pub fn dirty(&self) -> bool {
        self.bytes != self.baseline
    }

    pub fn remember(&mut self, path: &Path, music: &Music) -> Result<(), String> {
        if self.key.as_deref().is_some_and(|key| key != path) {
            return Err("The MIDI document changed".into());
        }
        self.bytes = smf_io::encode(music)?;
        self.auto_save = super::auto_save::enabled(music);
        if self.dirty() {
            self.dirty_since.get_or_insert_with(Instant::now);
        } else {
            self.dirty_since = None;
        }
        Ok(())
    }

    pub(crate) fn auto_save_deadline(&self) -> Option<Instant> {
        (self.auto_save_enabled() && !self.auto_save_stopped && self.dirty())
            .then(|| {
                self.dirty_since
                    .map(|start| start + super::auto_save::DELAY)
            })
            .flatten()
    }

    pub(crate) fn auto_save_enabled(&self) -> bool {
        self.saved_auto_save && self.auto_save
    }

    pub(crate) fn auto_save_failed(&mut self, error: String) {
        self.auto_save_stopped = true;
        self.error = Some(error);
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.key.as_ref().ok_or("No MIDI document is loaded")?;
        if !self.dirty() {
            self.auto_save_stopped = false;
            self.error = None;
            return Ok(());
        }
        let directory = self
            .directory
            .as_ref()
            .ok_or("Backup directory is unavailable")?;
        super::saved_write::replace_with_backup(
            path,
            &self.bytes,
            &self.expected,
            &directory.join("backup"),
        )
        .map_err(|e| format!("Cannot save {}: {e}", path.display()))?;
        self.expected = self.bytes.clone();
        self.baseline = self.bytes.clone();
        self.saved_auto_save = self.auto_save;
        self.dirty_since = None;
        self.auto_save_stopped = false;
        self.error = None;
        Ok(())
    }
}

impl crate::app::App {
    pub fn remember_loaded(&mut self) -> Result<(), String> {
        if let Some(loaded) = &self.loaded {
            self.saved.remember(&loaded.original, &loaded.music)?;
        }
        Ok(())
    }

    pub fn close(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui::ViewportCommand;
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if crate::view::opening::pending(self) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            return;
        }
        if !self.config.navigation.close_approved {
            self.request_close(ctx);
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            return;
        }
        if let Err(error) = self.state_cache.save() {
            self.config.navigation.close_approved = false;
            self.config.navigation.departure = Some(crate::view::files::Action::Close);
            self.config.navigation.departure_error = Some(error);
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }
    }

    pub(crate) fn request_close(&mut self, ctx: &eframe::egui::Context) {
        if self.allow_departure(crate::view::files::Action::Close) {
            // Both native close and the palette use the save/confirmation path.
            self.config.navigation.close_approved = true;
            crate::view::busy::request(self, ctx, crate::view::busy::Task::Close);
        }
    }
}

#[cfg(test)]
mod tests;
