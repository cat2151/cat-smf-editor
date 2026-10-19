use super::App;
use cat_smf_editor::edit::tracks;

impl App {
    pub(crate) fn new_smf(&mut self, ctx: &eframe::egui::Context) {
        if self.allow_departure(crate::view::files::Action::New) {
            crate::view::busy::request(self, ctx, crate::view::busy::Task::New);
        }
    }

    pub(crate) fn new_smf_unchecked(&mut self) {
        let music = tracks::new_song();
        let document =
            match crate::store::saved::Document::untitled(self.saved.directory.clone(), &music) {
                Ok(document) => document,
                Err(error) => {
                    self.load_error = Some(error);
                    return;
                }
            };
        if self.install_document(
            "untitled.mid".into(),
            "untitled.mid".into(),
            music,
            document,
        ) {
            self.file_open_requested = false;
            crate::play::playback::start(self, crate::play::playback::Mode::Track);
        }
    }

    pub(crate) fn add_track(&mut self) -> Result<(), String> {
        let before = self
            .loaded
            .as_ref()
            .ok_or("No SMF is loaded")?
            .music
            .clone();
        let mut music = before.clone();
        tracks::add(&mut music)?;
        cat_smf_editor::edit::regenerate(&mut music)?;
        let sound = self.resolve_sound(&music)?;
        // Stop the old track before applying the new selection; no old playback resumes.
        self.saved
            .remember(&self.loaded.as_ref().unwrap().original, &music)?;
        crate::play::transport::stop(self);
        self.apply_volume_music(music)?;
        self.audition.clear_gains();
        for (track, gain) in sound.volumes.into_iter().enumerate() {
            self.audition.set_gain(track, gain);
        }
        self.audition.set_master(sound.master);
        self.loaded.as_mut().unwrap().patches = sound.patches;
        self.song_id = self.song_id.wrapping_add(1);
        self.undo.push(before);
        self.select_track(self.loaded.as_ref().unwrap().music.tracks.len() - 1);
        crate::play::playback::start(self, crate::play::playback::Mode::Track);
        Ok(())
    }

    /// Bulk event edits can replace sound metadata and change track indices.
    pub(crate) fn edit_tracks(
        &mut self,
        edit: impl FnOnce(&mut cat_smf_editor::model::Music) -> Result<(), String>,
    ) -> Result<(), String> {
        let before = self
            .loaded
            .as_ref()
            .ok_or("No SMF is loaded")?
            .music
            .clone();
        let mut music = before.clone();
        edit(&mut music)?;
        cat_smf_editor::smf_io::encode(&music)?;
        let sound = self.resolve_sound(&music)?;
        self.saved
            .remember(&self.loaded.as_ref().unwrap().original, &music)?;
        crate::play::transport::stop(self);
        self.apply_volume_music(music)?;
        self.audition.reset();
        self.audition.clear_gains();
        for (track, gain) in sound.volumes.into_iter().enumerate() {
            self.audition.set_gain(track, gain);
        }
        self.audition.set_master(sound.master);
        self.loaded.as_mut().unwrap().patches = sound.patches;
        self.playback.mixer_track = self
            .playback
            .mixer_track
            .min(self.loaded.as_ref().unwrap().music.tracks.len() - 1);
        self.song_id = self.song_id.wrapping_add(1);
        self.undo.push(before);
        Ok(())
    }

    pub(crate) fn sync_saved_path(&mut self) {
        let Some(path) = self.saved.path().map(std::path::Path::to_owned) else {
            return;
        };
        let Some(loaded) = &mut self.loaded else {
            return;
        };
        if loaded.original == path {
            return;
        }
        loaded.name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        loaded.original = path.clone();
        self.recent.history.record(&path, &loaded.music);
        self.config.navigation.last_mid = Some(path);
        self.save_session();
        self.config.persist();
    }
}

#[cfg(test)]
mod tests;
