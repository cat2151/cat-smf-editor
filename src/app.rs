mod creation;
mod loading;
pub(crate) mod sound;
mod startup;
pub(crate) mod undo;
use cat_smf_editor::{model::Music, score::musical_time::MusicalTime, score::note_view::NoteView};
use eframe::egui;

pub struct App {
    pub startup: startup::State,
    pub palette: crate::view::palette::State,
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
    pub voicevox: crate::view::voicevox::State,
    pub analysis: crate::view::analysis::State,
    pub event_selection: crate::view::event_selection::State,
    pub mml_input: crate::view::mml_input::State,
    pub busy: crate::view::busy::State,
    pub opening: crate::view::opening::State,
    pub note_edit: Option<crate::view::note_edit::Editing>,
    pub trill_edit: Option<crate::view::trill_edit::Editing>,
    /// Set by `e` / `.`; the row drawing the cursor's note opens this field.
    pub note_edit_request: Option<crate::view::note_edit::Field>,
    pub edit_menu: crate::view::edit_menu::State,
    pub piano_roll: crate::view::piano_roll::State,
    pub staff: crate::view::staff::State,
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
    pub mascot: crate::play::mascot::State,
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
    pub(crate) fn empty() -> Self {
        Self {
            startup: Default::default(),
            palette: Default::default(),
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
            voicevox: Default::default(),
            analysis: crate::view::analysis::State::default(),
            event_selection: Default::default(),
            mml_input: Default::default(),
            busy: Default::default(),
            opening: Default::default(),
            note_edit: None,
            trill_edit: None,
            note_edit_request: None,
            edit_menu: crate::view::edit_menu::State::default(),
            piano_roll: crate::view::piano_roll::State::default(),
            staff: crate::view::staff::State::default(),
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
            mascot: Default::default(),
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
        self.refresh_display();
    }
    pub(crate) fn refresh_display(&mut self) {
        self.track_titles = self
            .music
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{} · {}", i + 1, t.name().unwrap_or_else(|| "—".into())))
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
        cat_smf_editor::startup_trace::publish();
        if cat_smf_editor::startup_trace::quit_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        #[cfg(windows)]
        let started = std::time::Instant::now();
        self.log_errors();
        if !crate::view::opening::pending(self) {
            crate::view::recent::prefetch(self, ctx);
        }
        crate::view::show(self, ctx);
        if !self.player_selection.setup.is_active() {
            crate::view::files::open_dialog(self, ctx, frame);
            crate::view::voicevox::pick_dialog(self, ctx, frame);
        }
        self.save_session();
        self.close(ctx);
        crate::view::busy::show(self, ctx);
        self.log_errors();
        #[cfg(windows)]
        crate::player::resident_link::trace_frame(self, started);
    }

    /// editor が起動した外部プロセスを落とす。
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::play::mascot::shut_down(self);
        crate::play::sung::prerender::shut_down(self);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if !self.player_selection.setup.is_active() {
            drop_tab(raw_input);
        }
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
