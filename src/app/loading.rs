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
            self.load_and_play(path);
        }
    }

    pub(crate) fn load_and_play(&mut self, path: PathBuf) {
        let before = self.song_id;
        self.load_mid_unchecked(path);
        if self.song_id != before {
            crate::play::playback::start(self, crate::play::playback::Mode::AllTracks);
        }
    }

    pub(crate) fn load_mid_unchecked(&mut self, path: PathBuf) {
        self.invalid_smf_name = None;
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut document = crate::store::saved::Document::new(self.saved.directory.clone());
        match document.load(&path) {
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
        let super::sound::Sound {
            patches,
            master,
            volumes,
        } = match self.resolve_sound(&music) {
            Ok(sound) => sound,
            Err(error) => {
                self.load_error = Some(error);
                return false;
            }
        };
        self.song_id = self.song_id.wrapping_add(1);
        self.undo.clear();
        self.layer_editor = Default::default();
        self.effect_editor = Default::default();
        self.note_edit = None;
        self.trill_edit = None;
        self.note_edit_request = None;
        self.edit_menu = Default::default();
        self.comma_menu = Default::default();
        self.piano_roll.open = false;
        self.tempo_curve.open = false;
        self.track_view = crate::cursor::split::State::default();
        self.audition.clear_gains();
        self.player_selection.open = false;
        self.audition.reset();
        self.playback.repeat = None;
        self.playback.pending = None;
        self.playback.active = None;
        self.load_error = None;
        self.invalid_smf_name = None;
        self.help_open = false;
        self.playback.mixer_open = false;
        self.select_track(0);
        self.generation += 1;
        for (track, gain) in volumes.into_iter().enumerate() {
            self.audition.set_gain(track, gain);
        }
        self.audition.set_master(master);
        let time = MusicalTime::new(&music);
        let track_titles = music
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                format!(
                    "{} · {}",
                    index + 1,
                    track.name().unwrap_or_else(|| "—".into())
                )
            })
            .collect();
        self.loaded = Some(Loaded {
            patches,
            original: path.clone(),
            display_rows: music
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    cat_smf_editor::score::display_rows::DisplayRows::new(
                        &t.events,
                        &NoteView::new(&t.events).rows,
                        time.map(i),
                    )
                })
                .collect(),
            name,
            keys: cat_smf_editor::score::key_signature::KeySignatures::new(&music),
            time_columns: music
                .tracks
                .iter()
                .enumerate()
                .map(|(i, t)| crate::view::time_columns::TimeColumns::new(&t.events, time.map(i)))
                .collect(),
            note_views: music
                .tracks
                .iter()
                .map(|t| NoteView::new(&t.events))
                .collect(),
            music,
            time,
            track_titles,
        });
        if !document.is_untitled() {
            let loaded = self.loaded.as_ref().unwrap();
            self.recent.history.record(&loaded.original, &loaded.music);
            self.config.navigation.last_mid = Some(path.clone());
        }
        self.saved = document;
        self.config.persist();
        true
    }
}
