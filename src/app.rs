mod loading;
mod sound;
pub(crate) mod undo;
use cat_smf_editor::{model::Music, score::musical_time::MusicalTime, score::note_view::NoteView};
use eframe::egui;

pub struct App {
    pub undo: undo::History,
    pub file_open_requested: bool,
    pub recent: crate::view::recent::State,
    pub song_id: u64,
    pub layer_editor: crate::view::layers::State,
    pub effect_editor: crate::view::effects::State,
    pub state_cache: cat_smf_editor::score::state_cache::Cache,
    pub patch_roots: crate::player::patch_roots::Cache,
    pub error_log: crate::store::error_log::Log,
    pub player_selection: crate::player::selection::State,
    pub saved: crate::store::saved::Document,
    pub settings_error: Option<String>,
    pub config: crate::store::config::Store,
    pub comma_menu: crate::view::comma_menu::State,
    pub analysis: crate::view::analysis::State,
    pub note_edit: Option<crate::view::note_edit::Editing>,
    pub trill_edit: Option<crate::view::trill_edit::Editing>,
    /// Set by `e` / `.`; the row drawing the cursor's note opens this field.
    pub note_edit_request: Option<crate::view::note_edit::Field>,
    pub edit_menu: crate::view::edit_menu::State,
    pub piano_roll: crate::view::piano_roll::State,
    pub tempo_curve: crate::view::tempo_curve::State,
    pub last_command: Option<crate::view::edit_menu::Command>,
    pub loaded: Option<Loaded>,
    pub load_error: Option<String>,
    pub invalid_smf_name: Option<String>,
    pub track: usize,
    pub track_view: crate::cursor::split::State,
    pub selected_event: Option<usize>,
    pub reset_scroll: bool,
    pub generation: u64,
    pub note_mode: bool,
    /// Note rows draw a dark keyboard behind their cells instead of beside them.
    pub help_open: bool,
    pub cursor_row: Option<usize>,
    pub cursor_position: Option<cat_smf_editor::score::musical_time::Position>,
    pub reveal_cursor: bool,
    pub audition: crate::play::audition::Audition,
    pub playback: crate::play::playback::Playback,
}
pub struct Loaded {
    pub patches: Vec<cat_smf_editor::score::instrument::Patch>,
    pub original: std::path::PathBuf,
    pub name: String,
    pub music: Music,
    pub time: MusicalTime,
    pub keys: cat_smf_editor::score::key_signature::KeySignatures,
    pub track_titles: Vec<String>,
    pub note_views: Vec<NoteView>,
    pub time_columns: Vec<crate::view::time_columns::TimeColumns>,
    pub display_rows: Vec<cat_smf_editor::score::display_rows::DisplayRows>,
}
impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings_error: Option<String>,
        player: Option<std::path::PathBuf>,
        mid: Option<std::path::PathBuf>,
    ) -> Self {
        // Add a Windows font for Japanese paths without bundling a font license.
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
        cc.egui_ctx.set_fonts(fonts);
        let mut app = Self::empty();
        app.config = crate::store::config::Store::local();
        app.settings_error = settings_error;
        app.recent.history = crate::store::recent::History::local();
        app.player_selection.executable = player;
        app.open_initial(mid);
        if app.config.analysis_open {
            crate::view::analysis::toggle(&mut app);
        }
        crate::play::playback::start(&mut app, crate::play::playback::Mode::AllTracks);
        app
    }
    pub(crate) fn empty() -> Self {
        Self {
            undo: Default::default(),
            file_open_requested: false,
            recent: Default::default(),
            song_id: 0,
            layer_editor: Default::default(),
            effect_editor: Default::default(),
            state_cache: cat_smf_editor::score::state_cache::Cache::new(
                crate::store::config::directory(),
            ),
            patch_roots: crate::player::patch_roots::Cache::new(crate::store::config::directory()),
            error_log: crate::store::error_log::Log::default(),
            player_selection: crate::player::selection::State::default(),
            saved: crate::store::saved::Document::new(crate::store::config::directory()),
            settings_error: None,
            config: Default::default(),
            comma_menu: crate::view::comma_menu::State::default(),
            analysis: crate::view::analysis::State::default(),
            note_edit: None,
            trill_edit: None,
            note_edit_request: None,
            edit_menu: crate::view::edit_menu::State::default(),
            piano_roll: crate::view::piano_roll::State::default(),
            tempo_curve: crate::view::tempo_curve::State::default(),
            last_command: None,
            loaded: None,
            load_error: None,
            invalid_smf_name: None,
            track: 0,
            track_view: crate::cursor::split::State::default(),
            selected_event: None,
            reset_scroll: true,
            generation: 0,
            note_mode: true,
            help_open: false,
            cursor_row: None,
            cursor_position: None,
            reveal_cursor: false,
            audition: crate::play::audition::Audition::default(),
            playback: crate::play::playback::Playback::default(),
        }
    }
    /// Applies an edit in memory and keeps the cursor position.
    pub fn edit_music(
        &mut self,
        edit: impl FnOnce(&mut Music) -> Result<(), String>,
    ) -> Result<(), String> {
        let loaded = self.loaded.as_ref().ok_or("No SMF is loaded")?;
        let before = loaded.music.clone();
        let mut music = before.clone();
        edit(&mut music)?;
        cat_smf_editor::edit::regenerate(&mut music)?;
        if cat_smf_editor::smf_io::encode(&before)? == cat_smf_editor::smf_io::encode(&music)? {
            return Ok(());
        }
        self.apply_music(music)?;
        self.undo.push(before);
        Ok(())
    }
    pub fn select_track(&mut self, track: usize) {
        if self.track != track {
            self.layer_editor.open = false;
            self.effect_editor.close_track();
        }
        self.player_selection.no_notes_error = false;
        if self.player_selection.open {
            self.player_selection.open = false;
        }
        if let Some(tracks) = &mut self.track_view.split {
            tracks[self.track_view.active] = track;
        }
        self.track_view.cursor_tick = None;
        self.track = track;
        self.selected_event = None;
        self.cursor_row = None;
        self.cursor_position = None;
        self.reveal_cursor = false;
        self.playback.open = false;
        self.reset_scroll = true;
    }
}
impl Loaded {
    pub fn refresh(&mut self) {
        self.time = MusicalTime::new(&self.music);
        self.keys = cat_smf_editor::score::key_signature::KeySignatures::new(&self.music);
        self.note_views = self
            .music
            .tracks
            .iter()
            .map(|t| NoteView::new(&t.events))
            .collect();
        self.display_rows = self
            .music
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                cat_smf_editor::score::display_rows::DisplayRows::new(
                    &t.events,
                    &self.note_views[i].rows,
                    self.time.map(i),
                )
            })
            .collect();
        self.time_columns = self
            .music
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| crate::view::time_columns::TimeColumns::new(&t.events, self.time.map(i)))
            .collect();
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(windows)]
        let started = std::time::Instant::now();
        self.log_errors();
        self.recent.cache.retain(|_, audio| {
            audio.pump();
            audio.is_prepared()
        });
        if !self.recent.cache.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(32));
        }
        crate::view::show(self, ctx);
        crate::view::files::open_dialog(self, ctx, frame);
        self.save_session();
        self.close(ctx);
        self.log_errors();
        #[cfg(windows)]
        crate::player::resident_link::trace_frame(self, started);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        drop_tab(raw_input);
    }
}

/// Removes Tab so egui never moves keyboard focus onto a widget, which would mute the view's shortcuts.
pub(crate) fn drop_tab(raw_input: &mut egui::RawInput) {
    raw_input.events.retain(|e| {
        !matches!(
            e,
            egui::Event::Key {
                key: egui::Key::Tab,
                ..
            }
        )
    });
}

#[cfg(test)]
mod tests;
