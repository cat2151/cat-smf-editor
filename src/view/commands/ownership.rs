use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
    Settings,
    Program,
    Edit,
    Go,
    Tracks,
    Split,
    Play,
    Mixer,
    Layers,
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
    Help,
    Recent,
    Confirm,
    Invalid,
    Waiting,
    Browser,
    Analysis,
    Voicevox,
    Singer,
}
impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "Score",
            Self::Settings => "Settings",
            Self::Program => "Program target",
            Self::Edit => "Edit",
            Self::Go => "Go",
            Self::Tracks => "Tracks",
            Self::Split => "Columns",
            Self::Play => "Play",
            Self::Mixer => "Mixer",
            Self::Layers | Self::LayerInput | Self::LayerHelp => "Layers",
            Self::Effects | Self::EffectHelp => "Effects",
            Self::Selection | Self::SelectionMenu => "Selection",
            Self::Mml => "MML",
            Self::Note => "Note",
            Self::Lyrics => "Lyrics",
            Self::Trill => "Trill",
            Self::Timeline => "Timeline",
            Self::Help => "Help",
            Self::Recent => "Recent",
            Self::Confirm => "File",
            Self::Invalid => "Invalid SMF",
            Self::Waiting => "Waiting",
            Self::Browser => "Browser",
            Self::Analysis => "Spectrum",
            Self::Voicevox => "VOICEVOX",
            Self::Singer => "VOICEVOX voice",
        }
    }
    pub fn owns_keys(self) -> bool {
        !matches!(
            self,
            Self::Main
                | Self::Mml
                | Self::LayerInput
                | Self::Note
                | Self::Singer
                | Self::Lyrics
                | Self::Trill
                | Self::Waiting
                | Self::Browser
        )
    }
}

pub fn owner(a: &App, ctx: &egui::Context) -> Screen {
    if ctx.viewport_id() != egui::ViewportId::ROOT {
        return Screen::Analysis;
    }
    if a.player_selection.setup.is_active()
        || super::super::busy::pending(a)
        || super::super::opening::pending(a)
    {
        return Screen::Waiting;
    }
    if a.invalid_smf_name.is_some() {
        return Screen::Invalid;
    }
    if a.config.navigation.departure.is_some() {
        return Screen::Confirm;
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
    if a.mml_input.open {
        return Screen::Mml;
    }
    if a.player_selection.open {
        return Screen::Browser;
    }
    if a.layer_editor.open {
        return a.layer_editor.command_screen();
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
    if a.playback.mixer_focused() {
        return Screen::Mixer;
    }
    if super::super::event_selection::valid(a) {
        return a.event_selection.command_screen();
    }
    if a.piano_roll.open || a.tempo_curve.open {
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
