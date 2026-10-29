use super::*;
use std::path::PathBuf;

impl App {
    pub fn open_mid(&mut self, path: PathBuf) {
        let path = match std::fs::canonicalize(&path) {
            Ok(path) => path,
            Err(error) => {
                self.load_error = Some(format!("{}: {error}", path.display()));
                return;
            }
        };
        if self.allow_departure(crate::view::files::Action::Load(path.clone())) {
            self.load_mid_unchecked(path);
        }
    }

    pub fn open_mid_and_play(&mut self, path: PathBuf) {
        let path = match std::fs::canonicalize(&path) {
            Ok(path) => path,
            Err(error) => {
                self.load_error = Some(format!("{}: {error}", path.display()));
                return;
            }
        };
        if self.allow_departure(crate::view::files::Action::LoadAndPlay(path.clone())) {
            crate::view::opening::request(self, path, true);
        }
    }

    pub(crate) fn load_and_play(&mut self, path: PathBuf) {
        crate::view::opening::request(self, path, true);
    }

    pub(crate) fn load_mid_unchecked(&mut self, path: PathBuf) {
        self.invalid_smf_name = None;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut document = crate::store::saved::Document::new(self.saved.directory.clone());
        match document.load_for_startup(&path, self.startup.pending) {
            Ok(music) => {
                self.install_document(path, name, music, document);
            }
            Err(error) => {
                if matches!(error, crate::store::saved::LoadError::InvalidSmf(_)) {
                    self.invalid_smf_name = Some(name);
                }
                self.load_error = Some(error.to_string());
            }
        }
    }
    pub(super) fn install_document(
        &mut self,
        path: PathBuf,
        name: String,
        music: Music,
        document: crate::store::saved::Document,
    ) -> bool {
        let sound = match self.resolve_sound(&music) {
            Ok(sound) => sound,
            Err(error) => {
                self.load_error = Some(error);
                return false;
            }
        };
        self.install_resolved(path, name, music, document, sound);
        true
    }

    pub(crate) fn install_resolved(
        &mut self,
        path: PathBuf,
        name: String,
        music: Music,
        document: crate::store::saved::Document,
        sound: super::sound::Sound,
    ) {
        let super::sound::Sound {
            patches,
            master,
            volumes,
        } = sound;
        self.song_id = self.song_id.wrapping_add(1);
        self.undo.clear();
        self.lifecycle = Default::default();
        self.edit_start = Default::default();
        self.edit_style = None;
        self.patterns = Default::default();
        self.macro_editor = Default::default();
        self.layer_editor = Default::default();
        self.drum_editor = Default::default();
        self.effect_editor = Default::default();
        self.note_edit = None;
        self.trill_edit = None;
        self.note_edit_request = None;
        self.edit_menu = Default::default();
        self.comma_menu = Default::default();
        self.piano_roll.open = false;
        self.tempo_curve.open = false;
        self.parameter_editor.close();
        self.track_view = crate::cursor::split::State::default();
        self.audition.clear_gains();
        self.player_selection.open = false;
        self.audition.reset();
        self.playback.repeat = None;
        self.playback.pending = None;
        self.playback.active = None;
        self.load_error = None;
        self.invalid_smf_name = None;
        self.playback.mixer_open = false;
        self.set_track(0);
        self.generation += 1;
        for (track, gain) in volumes.into_iter().enumerate() {
            self.audition.set_gain(track, gain);
        }
        self.audition.set_master(master);
        let display_span = cat_smf_editor::startup_trace::span(if self.startup.pending {
            "playback_indexes"
        } else {
            "score_display_build"
        });
        let time = MusicalTime::new(&music);
        let note_views: Vec<_> = music
            .tracks
            .iter()
            .map(|t| NoteView::new(&t.events))
            .collect();
        let expansions = crate::play::playback::expand_all(&music, &note_views);
        let mut loaded = Loaded {
            display_music: None,
            patches,
            original: path.clone(),
            name,
            keys: cat_smf_editor::score::key_signature::KeySignatures::new(&music),
            time,
            note_views,
            expansions,
            track_titles: Vec::new(),
            display_rows: Vec::new(),
            time_columns: Vec::new(),
            music,
        };
        if !self.startup.pending {
            loaded.refresh_display();
        }
        self.loaded = Some(loaded);
        if !self.startup.pending {
            crate::view::patterns::track_selected(self);
        }
        drop(display_span);
        let persist_span = (!self.startup.pending)
            .then(|| cat_smf_editor::startup_trace::span("history_status_write"));
        if !document.is_untitled() {
            let loaded = self.loaded.as_ref().unwrap();
            if !self.startup.pending {
                self.recent.history.record(&loaded.original, &loaded.music);
            }
            if self.config.navigation.last_mid.as_ref() != Some(&path) {
                self.config.session = None;
            }
            self.config.navigation.last_mid = Some(path.clone());
        }
        self.saved = document;
        if !self.startup.pending {
            self.config.persist();
        }
        drop(persist_span);
    }
}
