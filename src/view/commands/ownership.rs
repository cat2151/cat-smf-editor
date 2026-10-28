use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
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
    LayerHelp,
    Effects,
    EffectHelp,
    Selection,
    SelectionMenu,
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
}
impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "Event list",
            Self::EditStyle => "Edit style",
            Self::TrackRename => "Track name",
            Self::Settings => "Settings",
            Self::Program => "Program target",
            Self::Edit => "Edit",
            Self::Go => "Go",
            Self::Tracks => "Tracks",
            Self::Split => "Columns",
            Self::Play => "Play",
            Self::Mixer => "Mixer",
            Self::Layers | Self::LayerInput | Self::LayerHelp => "Layers",
            Self::Drums | Self::DrumInput | Self::DrumCreate => "Drum sequencer",
            Self::Effects | Self::EffectHelp => "Effects",
            Self::Selection | Self::SelectionMenu => "Selection",
            Self::Mml => "MML",
            Self::Note => "Note",
            Self::Lyrics => "Lyrics",
            Self::Trill => "Trill",
            Self::Timeline => "Timeline",
            Self::Overview => "Track Overview",
            Self::Help => "Help",
            Self::Recent => "Recent",
            Self::Confirm => "File",
            Self::Backup => "Backup",
            Self::Invalid => "Invalid SMF",
            Self::Missing => "Missing SMF",
            Self::Waiting => "Waiting",
            Self::Browser => "Browser",
            Self::Analysis => "Spectrum",
            Self::Voicevox => "VOICEVOX",
            Self::Singer => "VOICEVOX voice",
            Self::MacroEditor => "Macro edit",
            Self::ParameterPicker | Self::ParameterInput => "Automation",
            Self::Patterns => "Patterns",
            Self::Scene => "Scene",
            Self::Song => "Pattern / Song",
            Self::ArrangementInput => "Arrangement",
        }
    }
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
    if a.patterns.workspace.input_open() {
        return Screen::ArrangementInput;
    }
    if crate::view::patterns::workspace::focused_query(ctx).is_some() {
        return Screen::ArrangementInput;
    }
    if ctx.viewport_id() != egui::ViewportId::ROOT {
        if let Some(p) = crate::view::patterns::workspace::pane(ctx) {
            return if a.playback.open {
                Screen::Play
            } else {
                p.screen()
            };
        }
        return Screen::Analysis;
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
    if a.layer_editor.open {
        return a.layer_editor.command_screen();
    }
    if a.drum_editor.open {
        return if a.drum_editor.confirm_create {
            Screen::DrumCreate
        } else if a.drum_editor.input.is_some() {
            Screen::DrumInput
        } else {
            Screen::Drums
        };
    }
    if a.effect_editor.is_open() {
        return a.effect_editor.command_screen();
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
    if a.help_open {
        return Screen::Help;
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
    if super::super::event_selection::valid(a) {
        return a.event_selection.command_screen();
    }
    if a.piano_roll.open || a.tempo_curve.open || a.parameter_editor.open {
        return Screen::Timeline;
    }
    Screen::Main
}

pub fn reachable(a: &App, screen: Screen) -> bool {
    match screen {
        Screen::Settings | Screen::Program | Screen::Recent => true,
        Screen::Edit
        | Screen::Go
        | Screen::Tracks
        | Screen::Split
        | Screen::Play
        | Screen::Mixer
        | Screen::Layers
        | Screen::Effects
        | Screen::Timeline => a.loaded.is_some(),
        _ => false,
    }
}

pub fn enter(a: &mut App, ctx: &egui::Context, screen: Screen) {
    match screen {
        Screen::Settings | Screen::Program => {
            super::super::comma_menu::open(a);
            a.comma_menu.program = screen == Screen::Program;
        }
        Screen::Edit => a.edit_menu.open = true,
        Screen::Go | Screen::Tracks => {
            if !a.track_view.go_open {
                a.track_view.origin_tick = crate::play::playback::location(a).0;
            }
            a.track_view.go_open = true;
            a.track_view.track_list = screen == Screen::Tracks;
        }
        Screen::Split => a.track_view.split_open = true,
        Screen::Play => a.playback.open = true,
        Screen::Mixer => {
            crate::play::mixer::open(a);
            a.playback.mixer_master = false;
        }
        Screen::Layers => super::super::layers::open(a),
        Screen::Effects => crate::play::mixer::open_effects(a, false),
        Screen::Recent => super::super::recent::commands::open(a),
        Screen::Timeline => super::super::piano_roll::open(a),
        _ => {}
    }
    let _ = ctx;
}
