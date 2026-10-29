use super::commands::{Command, Screen};
use crate::app::App;
pub(crate) mod beacon;
mod cards;
mod first;
pub(crate) mod library;
pub(crate) mod matrix;
pub(crate) mod membership;
pub(crate) mod mode;
#[cfg(test)]
mod native_tests;
pub(crate) mod navigation;
pub(crate) mod presentation;
pub(crate) mod preview;
pub(crate) mod windows;
pub(crate) mod workspace;
pub struct PreviewTempo(pub f64);
impl Default for PreviewTempo {
    fn default() -> Self {
        Self(120.0)
    }
}
#[derive(Default)]
pub struct State {
    pub recording: crate::view::recording::State,
    pub workspace: workspace::State,
    pub navigation: navigation::State,
    first: Option<first::Pending>,
    pub selected: Option<u64>,
    pub container: Option<u64>,
    pub tempo: PreviewTempo,
    pub editing: Option<cat_smf_editor::score::pattern::editing::Context>,
    pub raw_context: Option<cat_smf_editor::score::pattern::editing::Context>,
    pub edit_song: u64,
    pub preview: Option<u64>,
    pub error: Option<String>,
    /// Explicit Scene / Song request for song destinations; Notes sources always open them.
    pub workspace_open: bool,
    pub root_focus: bool,
}

impl State {
    pub fn confirming_creation(&self) -> bool {
        self.first.is_some()
    }
}

pub fn refresh(a: &mut App) {
    let Some(loaded) = &a.loaded else { return };
    if let Some(edit) = &mut a.patterns.editing {
        if edit.rebind(&loaded.music).is_err() {
            a.patterns.editing = None;
        } else if edit.standalone {
            a.track = edit.track;
            if let Some(tracks) = &mut a.track_view.split {
                tracks[a.track_view.active] = a.track;
            }
        }
    }
    let workspace_open = mode::workspace_open(a);
    let loaded = a.loaded.as_mut().unwrap();
    loaded.display_music = a
        .patterns
        .editing
        .as_ref()
        .filter(|c| workspace_open && a.patterns.edit_song == a.song_id && c.track == a.track)
        .and_then(|context| context.project(&loaded.music).ok());
    if loaded.display_music.is_none() {
        a.patterns.editing = None;
    }
    loaded.keys = if loaded.display_music.is_some() {
        cat_smf_editor::score::key_signature::KeySignatures::local_timeline(&loaded.music)
    } else {
        cat_smf_editor::score::key_signature::KeySignatures::new(&loaded.music)
    };
    loaded.time = cat_smf_editor::score::musical_time::MusicalTime::new(loaded.visible());
    loaded.note_views = loaded
        .visible()
        .tracks
        .iter()
        .map(|t| cat_smf_editor::score::note_view::NoteView::new(&t.events))
        .collect();
    loaded.refresh_display();
    if let Some(edit) = &a.patterns.editing
        && edit.standalone
        && let (Some(map), Ok(range)) =
            (loaded.time.map(edit.track), edit.edit_range(&loaded.music))
    {
        let events = loaded.visible().tracks[edit.track].events.clone();
        loaded.display_rows[edit.track].clip_to_measure(&events, map, range);
    }
}

pub fn validate(a: &mut App) {
    crate::view::drums::reconcile(a);
    let Some(context) = &a.patterns.editing else {
        return;
    };
    let current = crate::play::playback::location(a).0;
    if a.patterns.edit_song != a.song_id
        || context.track != a.track
        || a.loaded
            .as_ref()
            .and_then(|l| context.edit_range(&l.music).ok())
            .is_none_or(|(start, end)| !(start..end).contains(&current))
    {
        let tick = crate::play::playback::location(a).0;
        a.patterns.editing = None;
        a.generation = a.generation.wrapping_add(1);
        refresh(a);
        crate::cursor::moves::at_tick(a, tick);
        crate::view::drums::reconcile(a);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Create,
    Duplicate,
    Preview,
    MoveContainer,
    Structure,
    AuditionTempo,
    MidiBaseline,
}
impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Self::Create => "create-pattern",
            Self::Duplicate => "duplicate-pattern",
            Self::Preview => "preview-pattern",
            Self::MoveContainer => "move-pattern-container",
            Self::Structure => "pattern-structure",
            Self::AuditionTempo => "pattern-audition-tempo",
            Self::MidiBaseline => "pattern-midi-baseline",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Create => "Add pattern",
            Self::Duplicate => "Duplicate pattern",
            Self::Preview => "Preview pattern",
            Self::MoveContainer => "Move pattern to container",
            Self::Structure => "Pattern measures / meter",
            Self::AuditionTempo => "Audition tempo",
            Self::MidiBaseline => "Container MIDI baseline",
        }
    }
}
pub(crate) fn collect(app: &App, s: Screen, out: &mut Vec<Command>) {
    workspace::collect(app, s, out);
    if app.patterns.confirming_creation() {
        if s == Screen::Patterns {
            first::collect(s, out);
        }
        return;
    }
    if !matches!(s, Screen::Main | Screen::Scene | Screen::Song) || app.loaded.is_none() {
        return;
    }
    out.push(Command::operation(
        s,
        "open-song-matrix",
        "Pattern / Song",
        super::commands::Operation::PatternWork,
    ));
    for action in [
        Action::Create,
        Action::Duplicate,
        Action::Preview,
        Action::MoveContainer,
        Action::Structure,
        Action::AuditionTempo,
        Action::MidiBaseline,
    ] {
        out.push(Command::operation(
            s,
            action.id(),
            action.label(),
            super::commands::Operation::pattern(action),
        ));
    }
}

pub fn show(a: &mut App, ctx: &eframe::egui::Context) {
    if a.patterns.confirming_creation() {
        first::show(a, ctx);
    }
}

pub use navigation::track_selected;

pub use presentation::heading;

#[cfg(test)]
pub(crate) mod edit_tests;

#[cfg(test)]
mod acceptance_tests;
#[cfg(test)]
mod beacon_tests;
#[cfg(test)]
mod e2e_tests;
#[cfg(test)]
mod feedback_tests;
#[cfg(test)]
mod ux_tests;

#[cfg(test)]
mod track_tests;

#[cfg(test)]
mod mode_tests;
#[cfg(test)]
mod mode_transition_tests;
#[cfg(test)]
mod raw_tests;
#[cfg(test)]
mod song_mode_tests;
#[cfg(test)]
pub(crate) mod workspace_tests;
