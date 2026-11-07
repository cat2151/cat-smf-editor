//! The status.json layout: what the app remembers between runs.
use super::{ProgramTarget, Session};
use cat_smf_editor::{
    patch_identity::PatchIdentity,
    score::{harmony, pattern::recording},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub(crate) struct Status {
    #[serde(default)]
    pub navigation: crate::store::navigation::Navigation,
    pub session: Option<Session>,
    #[serde(default)]
    pub program_target: ProgramTarget,
    #[serde(default = "enabled")]
    pub tempo_curve_maximized: bool,
    #[serde(default = "enabled")]
    pub parameter_curve_maximized: bool,
    #[serde(default = "enabled")]
    pub macro_variation_curve_maximized: bool,
    #[serde(default = "enabled")]
    pub piano_roll_maximized: bool,
    #[serde(default = "enabled")]
    pub meta_text_visible: bool,
    #[serde(default)]
    pub browser_panes: std::collections::BTreeMap<String, [String; 2]>,
    #[serde(default)]
    pub browser_queries: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub patch_cursors: std::collections::BTreeMap<String, PatchIdentity>,
    /// Spectrum window: outer x, y and inner width, height in logical pixels.
    #[serde(default)]
    pub analysis_window: Option<[f32; 4]>,
    #[serde(default)]
    pub analysis_open: bool,
    #[serde(default)]
    pub help_open: bool,
    #[serde(default)]
    pub help_window: Option<[f32; 4]>,
    /// Pattern companion windows: outer x/y and inner width/height.
    #[serde(default)]
    pub pattern_windows: std::collections::BTreeMap<String, [f32; 4]>,
    /// Embedded drum sequencers: top-left x/y in logical pixels.
    #[serde(default)]
    pub drum_windows: std::collections::BTreeMap<String, [f32; 2]>,
    /// Embedded drum sequencer content width/height in logical pixels.
    #[serde(default)]
    pub drum_window_sizes: std::collections::BTreeMap<String, [f32; 2]>,
    /// Virtual MIDI keyboard windows: outer x/y.
    #[serde(default)]
    pub keyboard_windows: std::collections::BTreeMap<String, [f32; 2]>,
    /// The app closed with the Virtual MIDI keyboard open and its phrase loop on.
    #[serde(default)]
    pub phrase_loop: bool,
    /// Per container: which pitches the step sequencer of its patterns shows as rows.
    #[serde(default)]
    pub melody_row_sets: std::collections::BTreeMap<u64, harmony::Set>,
    /// Per container: the step grid's row height relative to the unzoomed grid.
    #[serde(default)]
    pub melody_zooms: std::collections::BTreeMap<u64, f32>,
    /// The keyboard, not the window it opened from, had the focus last when the app closed.
    #[serde(default)]
    pub keyboard_focused: bool,
    /// Virtual MIDI keyboard settings and loop length the keyboard last closed with.
    #[serde(default)]
    pub keyboard_settings: Option<(recording::Settings, u16)>,
    /// What the pads of each container played, by container id.
    #[serde(default)]
    pub keyboard_voicings: std::collections::BTreeMap<u64, recording::Voicing>,
    /// VOICEVOX engine の `run.exe`。
    #[serde(default)]
    pub voicevox_engine: Option<PathBuf>,
    #[serde(default)]
    pub mascot_enabled: bool,
}

fn enabled() -> bool {
    true
}

impl Default for Status {
    fn default() -> Self {
        Self {
            navigation: Default::default(),
            session: None,
            program_target: ProgramTarget::default(),
            tempo_curve_maximized: true,
            parameter_curve_maximized: true,
            macro_variation_curve_maximized: true,
            piano_roll_maximized: true,
            meta_text_visible: true,
            browser_panes: Default::default(),
            browser_queries: Default::default(),
            patch_cursors: Default::default(),
            analysis_window: None,
            analysis_open: false,
            help_open: false,
            help_window: None,
            pattern_windows: Default::default(),
            drum_windows: Default::default(),
            drum_window_sizes: Default::default(),
            keyboard_windows: Default::default(),
            phrase_loop: false,
            melody_row_sets: Default::default(),
            melody_zooms: Default::default(),
            keyboard_focused: false,
            keyboard_settings: None,
            keyboard_voicings: Default::default(),
            voicevox_engine: None,
            mascot_enabled: false,
        }
    }
}
