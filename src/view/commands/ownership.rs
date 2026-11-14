use super::*;
mod navigation;
pub use navigation::{destinations, navigate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum Screen {
    /// どの画面にも属さず、どの画面のパレットからも実行できる。
    Global,
    Main,
    /// The score pane beside the event list.
    Score,
    EditStyle,
    TrackRename,
    Settings,
    Program,
    Edit,
    Go,
    Tracks,
    Split,
    Play,
    Mixer,
    Layers,
    Drums,
    DrumInput,
    DrumCreate,
    LayerInput,
    Effects,
    Selection,
    SelectionMenu,
    ChordTiming,
    Mml,
    Note,
    Lyrics,
    Trill,
    Timeline,
    Overview,
    Help,
    Recent,
    Confirm,
    Backup,
    Invalid,
    Missing,
    Waiting,
    Browser,
    Analysis,
    Voicevox,
    Singer,
    MacroEditor,
    ParameterPicker,
    ParameterInput,
    Patterns,
    Scene,
    Song,
    ArrangementInput,
    /// Virtual MIDI keyboard とその付属窓。
    Recording,
}
impl Screen {
    pub fn owns_keys(self) -> bool {
        !matches!(
            self,
            Self::Main
                | Self::ArrangementInput
                | Self::Scene
                | Self::Song
                | Self::TrackRename
                | Self::Mml
                | Self::LayerInput
                | Self::DrumInput
                | Self::Note
                | Self::Singer
                | Self::ParameterPicker
                | Self::ParameterInput
                | Self::Lyrics
                | Self::Trill
                | Self::Waiting
                | Self::Browser
        )
    }
}

pub fn owner(a: &App, ctx: &egui::Context) -> Screen {
    if a.player_selection.kit_open {
        return Screen::Waiting;
    }
    use crate::view::dock::{focus, locate, Host, ViewKind};
    let view = locate::view_at(a, ctx);
    if view == Some(ViewKind::Help) {
        return Screen::Help;
    }
    if a.patterns.workspace.input_open() {
        return Screen::ArrangementInput;
    }
    if crate::view::patterns::workspace::focused_query(ctx).is_some() {
        return Screen::ArrangementInput;
    }
    if view == Some(ViewKind::Keyboard) && root_owner(a).allows_companions_for(a) {
        return Screen::Recording;
    }
    match locate::host_at(a, ctx) {
        Some(Host::Root) => root_owner(a),
        Some(Host::Dock(_)) => {
            let root = root_owner(a);
            match view {
                Some(view) if root.allows_companions_for(a) => focus::screen(view),
                _ => root,
            }
        }
        None => match view {
            Some(ViewKind::Song | ViewKind::Scene) if a.playback.open => Screen::Play,
            Some(view @ (ViewKind::Song | ViewKind::Scene)) => focus::screen(view),
            _ => Screen::Analysis,
        },
    }
}

/// No input or menu of the root window holds a row of what it shows.
pub fn root_idle(a: &App) -> bool {
    matches!(
        root_owner(a),
        Screen::Main
            | Screen::Score
            | Screen::Drums
            | Screen::Timeline
            | Screen::Mixer
            | Screen::Play
            | Screen::Overview
    )
}

/// The screen owning the root window, whichever window has focus.
pub(crate) fn root_owner(a: &App) -> Screen {
    if a.patterns.workspace.input_open() {
        return Screen::ArrangementInput;
    }
    if a.player_selection.setup.is_active()
        || super::super::busy::pending(a)
        || super::super::opening::pending(a)
    {
        return Screen::Waiting;
    }
    if a.missing_last_mid.is_some() {
        return Screen::Missing;
    }
    if a.invalid_smf_name.is_some() {
        return Screen::Invalid;
    }
    if a.config.navigation.departure.is_some() {
        return Screen::Confirm;
    }
    if a.config.navigation.backup_notice {
        return Screen::Backup;
    }
    if a.edit_style.is_some() {
        return Screen::EditStyle;
    }
    if a.track_rename.is_some() {
        return Screen::TrackRename;
    }
    if a.macro_editor.open() {
        return Screen::MacroEditor;
    }
    if a.parameter_editor.picker_open() {
        return Screen::ParameterPicker;
    }
    if a.parameter_editor.input_open() {
        return Screen::ParameterInput;
    }
    if a.voicevox.wizard {
        return Screen::Voicevox;
    }
    if a.voicevox.singer.is_some() {
        return Screen::Singer;
    }
    if a.recent.open {
        return Screen::Recent;
    }
    if a.trill_edit.is_some() {
        return Screen::Trill;
    }
    if a.note_edit.is_some() {
        return Screen::Note;
    }
    if a.voicevox.lyrics.is_some() {
        return Screen::Lyrics;
    }
    if a.patterns.confirming_creation() {
        return Screen::Patterns;
    }
    if a.mml_input.open {
        return Screen::Mml;
    }
    if a.player_selection.open {
        return Screen::Browser;
    }
    if super::super::event_selection::valid(a) {
        return a.event_selection.command_screen();
    }
    if a.layer_editor.open {
        return a.layer_editor.command_screen();
    }
    if a.effect_editor.is_open() {
        return a.effect_editor.command_screen();
    }
    if crate::view::drums::is_open(a) {
        if a.drum_editor.confirm_create {
            return Screen::DrumCreate;
        }
        if a.drum_editor.input.is_some() || a.drum_editor.kit.has_input() {
            return Screen::DrumInput;
        }
        // A grid placed in a pane stays open beside the pane holding the keys.
        use crate::view::dock::{focus, ViewKind};
        if !focus::placed(a, ViewKind::StepSequencer)
            || focus::focused(a) == ViewKind::StepSequencer
        {
            return Screen::Drums;
        }
    }
    if a.comma_menu.open {
        return if a.comma_menu.program {
            Screen::Program
        } else {
            Screen::Settings
        };
    }
    if a.edit_menu.open {
        return Screen::Edit;
    }
    if a.track_view.go_open {
        return if a.track_view.track_list {
            Screen::Tracks
        } else {
            Screen::Go
        };
    }
    if a.track_view.split_open {
        return Screen::Split;
    }
    if a.playback.open {
        return Screen::Play;
    }
    if a.track_overview.open {
        return Screen::Overview;
    }
    if a.playback.mixer_focused() {
        return Screen::Mixer;
    }
    if super::super::piano_roll::chord_timing::active(a) {
        return Screen::ChordTiming;
    }
    if a.piano_roll.open || a.tempo_curve.open || a.parameter_editor.open {
        return Screen::Timeline;
    }
    crate::view::dock::focus::screen(crate::view::dock::focus::focused(a))
}

impl Screen {
    /// Input windows and dialogs: leaving them would strand the input, so their palette shows only them.
    pub fn transient(self) -> bool {
        matches!(
            self,
            Self::EditStyle
                | Self::TrackRename
                | Self::LayerInput
                | Self::DrumInput
                | Self::DrumCreate
                | Self::Selection
                | Self::SelectionMenu
                | Self::ChordTiming
                | Self::Mml
                | Self::Note
                | Self::Lyrics
                | Self::Trill
                | Self::Confirm
                | Self::Backup
                | Self::Invalid
                | Self::Missing
                | Self::Waiting
                | Self::Browser
                | Self::Voicevox
                | Self::Singer
                | Self::MacroEditor
                | Self::ParameterPicker
                | Self::ParameterInput
                | Self::Patterns
                | Self::ArrangementInput
        )
    }
    /// App-wide dialogs, where even screen-independent commands are withheld.
    pub fn app_dialog(self) -> bool {
        matches!(
            self,
            Self::Waiting
                | Self::Missing
                | Self::Invalid
                | Self::Confirm
                | Self::Backup
                | Self::EditStyle
        )
    }
    fn pane(self) -> Option<crate::view::patterns::workspace::Pane> {
        use crate::view::patterns::workspace::Pane;
        match self {
            Self::Scene => Some(Pane::Scene),
            Self::Song => Some(Pane::Song),
            _ => None,
        }
    }
    /// The command leaving this root screen towards the Event list.
    pub(super) fn close(self) -> Option<&'static str> {
        Some(match self {
            Self::Settings => "settings.close",
            Self::Program => "program.close",
            Self::Edit => "edit.close",
            Self::Go => "go.close",
            Self::Tracks => "tracks.close",
            Self::Split => "split.close",
            Self::Play => "play.close",
            Self::Mixer => "mixer.unfocus",
            Self::Score => "pane.unfocus",
            Self::Layers => "layer.close",
            Self::Effects => "effect.close",
            Self::Recent => "recent.close",
            Self::Timeline => "timeline.close",
            Self::Overview => "overview.close",
            Self::Drums => "drum.cancel",
            _ => return None,
        })
    }
}
