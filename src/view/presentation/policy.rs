use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    Workspace,
    FocusedWorkspace,
    Operation,
}

impl Screen {
    /// Exhaustive on purpose: adding an input owner requires a presentation decision.
    fn presentation(self) -> Policy {
        match self {
            Self::Main
            | Self::Score
            | Self::Timeline
            | Self::Mixer
            | Self::Drums
            | Self::Scene
            | Self::Song
            | Self::Recording
            | Self::Analysis
            | Self::Help
            | Self::Selection => Policy::Workspace,
            Self::Overview => Policy::FocusedWorkspace,
            Self::Global
            | Self::EditStyle
            | Self::TrackRename
            | Self::Settings
            | Self::Program
            | Self::Edit
            | Self::Go
            | Self::Tracks
            | Self::Split
            | Self::Play
            | Self::Layers
            | Self::DrumInput
            | Self::DrumCreate
            | Self::LayerInput
            | Self::Effects
            | Self::SelectionMenu
            | Self::ChordTiming
            | Self::Mml
            | Self::Note
            | Self::Lyrics
            | Self::Trill
            | Self::Recent
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
            | Self::ArrangementInput => Policy::Operation,
        }
    }
    pub(crate) fn allows_companions(self) -> bool {
        self.presentation() == Policy::Workspace
    }
    /// A placed overview is a workspace; its floating window keeps exclusive input.
    pub(crate) fn allows_companions_for(self, a: &App) -> bool {
        self.allows_companions()
            || (self == Self::Overview
                && crate::view::dock::focus::placed(a, crate::view::dock::ViewKind::TrackOverview))
    }
    pub(super) fn floating_order(self) -> egui::Order {
        match self.presentation() {
            Policy::Workspace | Policy::FocusedWorkspace => egui::Order::Middle,
            Policy::Operation => egui::Order::Foreground,
        }
    }
}
