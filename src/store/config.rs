use cat_smf_editor::patch_identity::PatchIdentity;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn directory() -> Option<PathBuf> {
    cat_smf_editor::storage_paths::directory()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub file: String,
    /// One-based, like the track list.
    pub track: usize,
    pub mbt: Option<String>,
    pub tick: Option<u64>,
    pub row: Option<usize>,
    pub note_mode: bool,
    /// Left and right tracks of two columns, one-based like `track`.
    #[serde(default)]
    pub split: Option<[usize; 2]>,
    /// Focused slot of `split`; both slots may show the same track.
    #[serde(default)]
    pub split_active: usize,
}

/// Which tracks `t` changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramTarget {
    /// All note tracks when they share one program name, otherwise the current track.
    #[default]
    Auto,
    Current,
}

#[derive(Serialize, Deserialize)]
pub struct Status {
    #[serde(default)]
    pub navigation: super::navigation::Navigation,
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
    /// Virtual MIDI keyboard windows: outer x/y.
    #[serde(default)]
    pub keyboard_windows: std::collections::BTreeMap<String, [f32; 2]>,
    /// VOICEVOX engine の `run.exe`。
    #[serde(default)]
    pub voicevox_engine: Option<PathBuf>,
    #[serde(default)]
    pub mascot_enabled: bool,
}

pub struct Store {
    pub(crate) persisted_mid: Option<PathBuf>,
    /// Document belonging to the deferred UI snapshot, independent of startup.json.
    pub(crate) session_document: Option<PathBuf>,
    pub(crate) read_failed: bool,
    pub navigation: super::navigation::Navigation,
    pub path: Option<PathBuf>,
    pub session: Option<Session>,
    pub program_target: ProgramTarget,
    pub tempo_curve_maximized: bool,
    pub parameter_curve_maximized: bool,
    pub macro_variation_curve_maximized: bool,
    pub piano_roll_maximized: bool,
    pub meta_text_visible: bool,
    pub browser_panes: std::collections::BTreeMap<String, [String; 2]>,
    pub browser_queries: std::collections::BTreeMap<String, String>,
    pub patch_cursors: std::collections::BTreeMap<String, PatchIdentity>,
    pub analysis_window: Option<[f32; 4]>,
    pub analysis_open: bool,
    pub help_open: bool,
    pub help_window: Option<[f32; 4]>,
    pub pattern_windows: std::collections::BTreeMap<String, [f32; 4]>,
    pub keyboard_windows: std::collections::BTreeMap<String, [f32; 2]>,
    pub voicevox_engine: Option<PathBuf>,
    pub mascot_enabled: bool,
    pub error: Option<String>,
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
            keyboard_windows: Default::default(),
            voicevox_engine: None,
            mascot_enabled: false,
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self {
            persisted_mid: None,
            session_document: None,
            read_failed: false,
            navigation: Default::default(),
            path: None,
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
            keyboard_windows: Default::default(),
            voicevox_engine: None,
            mascot_enabled: false,
            error: None,
        }
    }
}

impl Store {
    pub fn local() -> Self {
        match directory() {
            Some(root) => Self::open_directory(root),
            None => Self {
                error: Some("Cannot save session: LOCALAPPDATA is unavailable".into()),
                ..Self::default()
            },
        }
    }

    pub fn open_directory(root: PathBuf) -> Self {
        let path = root.join("status.json");
        let legacy = root.join("config.json");
        let migration = (|| -> std::io::Result<()> {
            if !path.try_exists()? && legacy.try_exists()? {
                std::fs::rename(&legacy, &path)?;
            }
            Ok(())
        })();
        let mut store = Self::open(path);
        if let Err(error) = migration {
            store.error = Some(format!("Cannot rename config.json to status.json: {error}"));
        }
        store
    }

    pub fn open(path: PathBuf) -> Self {
        let mut store = Self {
            path: Some(path),
            ..Self::default()
        };
        let path = store.path.as_ref().unwrap();
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(Status {
                    navigation,
                    session,
                    program_target,
                    tempo_curve_maximized,
                    parameter_curve_maximized,
                    macro_variation_curve_maximized,
                    piano_roll_maximized,
                    meta_text_visible,
                    browser_panes,
                    browser_queries,
                    patch_cursors,
                    analysis_window,
                    analysis_open,
                    help_open,
                    help_window,
                    pattern_windows,
                    keyboard_windows,
                    voicevox_engine,
                    mascot_enabled,
                }) => {
                    store.navigation = navigation;
                    store.session = session;
                    store.program_target = program_target;
                    store.tempo_curve_maximized = tempo_curve_maximized;
                    store.parameter_curve_maximized = parameter_curve_maximized;
                    store.macro_variation_curve_maximized = macro_variation_curve_maximized;
                    store.piano_roll_maximized = piano_roll_maximized;
                    store.meta_text_visible = meta_text_visible;
                    store.browser_panes = browser_panes;
                    store.browser_queries = browser_queries;
                    store.patch_cursors = patch_cursors;
                    store.analysis_window = analysis_window.filter(|window| {
                        window.iter().all(|n| n.is_finite()) && window[2] > 0.0 && window[3] > 0.0
                    });
                    store.analysis_open = analysis_open;
                    store.help_open = help_open;
                    store.help_window = help_window
                        .filter(|w| w.iter().all(|n| n.is_finite()) && w[2] > 0.0 && w[3] > 0.0);
                    store.pattern_windows = pattern_windows
                        .into_iter()
                        .filter(|(_, w)| {
                            w.iter().all(|v| v.is_finite()) && w[2] > 0.0 && w[3] > 0.0
                        })
                        .collect();
                    store.keyboard_windows = keyboard_windows
                        .into_iter()
                        .filter(|(_, at)| at.iter().all(|v| v.is_finite()))
                        .collect();
                    store.voicevox_engine = voicevox_engine;
                    store.mascot_enabled = mascot_enabled;
                }
                Err(error) => {
                    store.error = Some(format!("Cannot read {}: {error}", path.display()))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => store.error = Some(format!("Cannot read {}: {error}", path.display())),
        }
        store.session_document = store.navigation.last_mid.clone();
        match super::startup::read(path) {
            Ok(startup) => {
                store.navigation.last_mid = startup.last_mid.clone();
                store.persisted_mid = startup.last_mid;
            }
            Err(error) => store.error = Some(error),
        }
        store.read_failed = store.error.is_some();
        store
    }

    pub fn save(&mut self, session: Session) {
        if self.session.as_ref() == Some(&session) {
            return;
        }
        self.session = Some(session);
        self.persist();
    }

    pub fn select_program_target(&mut self, target: ProgramTarget) {
        self.program_target = target;
        self.persist();
    }

    pub fn select_tempo_curve_maximized(&mut self, maximized: bool) {
        self.tempo_curve_maximized = maximized;
        self.persist();
    }

    pub fn select_parameter_curve_maximized(&mut self, maximized: bool) {
        self.parameter_curve_maximized = maximized;
        self.persist();
    }

    pub fn select_macro_variation_curve_maximized(&mut self, maximized: bool) {
        self.macro_variation_curve_maximized = maximized;
        self.persist();
    }

    pub fn select_piano_roll_maximized(&mut self, maximized: bool) {
        self.piano_roll_maximized = maximized;
        self.persist();
    }

    pub fn select_meta_text_visible(&mut self, visible: bool) {
        self.meta_text_visible = visible;
        self.persist();
    }

    pub fn select_browser_panes(&mut self, context: String, panes: [String; 2], query: String) {
        if self.browser_panes.get(&context) != Some(&panes)
            || self.browser_queries.get(&context) != Some(&query)
        {
            self.browser_panes.insert(context.clone(), panes);
            self.browser_queries.insert(context, query);
            self.persist();
        }
    }

    pub fn select_patch_cursor(&mut self, context: String, patch: Option<PatchIdentity>) {
        if self.patch_cursors.get(&context) == patch.as_ref() {
            return;
        }
        if let Some(patch) = patch {
            self.patch_cursors.insert(context, patch);
        } else {
            self.patch_cursors.remove(&context);
        }
        self.persist();
    }

    pub fn select_analysis_window(&mut self, window: [f32; 4]) {
        if self.analysis_window != Some(window) {
            self.analysis_window = Some(window);
            self.persist();
        }
    }

    pub fn forget_analysis_window(&mut self) {
        if self.analysis_window.take().is_some() {
            self.persist();
        }
    }

    pub fn select_analysis_open(&mut self, open: bool) {
        if self.analysis_open != open {
            self.analysis_open = open;
            self.persist();
        }
    }

    pub fn select_voicevox_engine(&mut self, path: PathBuf) {
        if self.voicevox_engine.as_ref() != Some(&path) {
            self.voicevox_engine = Some(path);
            self.persist();
        }
    }

    pub fn select_mascot_enabled(&mut self, enabled: bool) {
        if self.mascot_enabled != enabled {
            self.mascot_enabled = enabled;
            self.persist();
        }
    }

    pub(crate) fn persist(&mut self) {
        if self.read_failed {
            return;
        }
        let Some(path) = &self.path else {
            return;
        };
        if self.persisted_mid != self.navigation.last_mid {
            if let Err(error) = super::startup::save(path, self.navigation.last_mid.clone()) {
                self.error = Some(error);
                return;
            }
            self.persisted_mid = self.navigation.last_mid.clone();
        }
        let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let bytes = serde_json::to_vec_pretty(&Status {
                navigation: super::navigation::Navigation {
                    last_mid: self.navigation.last_mid.clone(),
                    backup_tip_date: self.navigation.backup_tip_date.clone(),
                    ..Default::default()
                },
                session: self.session.clone(),
                program_target: self.program_target,
                tempo_curve_maximized: self.tempo_curve_maximized,
                parameter_curve_maximized: self.parameter_curve_maximized,
                macro_variation_curve_maximized: self.macro_variation_curve_maximized,
                piano_roll_maximized: self.piano_roll_maximized,
                meta_text_visible: self.meta_text_visible,
                browser_panes: self.browser_panes.clone(),
                browser_queries: self.browser_queries.clone(),
                patch_cursors: self.patch_cursors.clone(),
                analysis_window: self.analysis_window,
                analysis_open: self.analysis_open,
                help_open: self.help_open,
                help_window: self.help_window,
                pattern_windows: self.pattern_windows.clone(),
                keyboard_windows: self.keyboard_windows.clone(),
                voicevox_engine: self.voicevox_engine.clone(),
                mascot_enabled: self.mascot_enabled,
            })?;
            // Keep the previous status intact until the replacement is complete.
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        } else {
            self.session_document = self.navigation.last_mid.clone();
        }
        self.error = result
            .err()
            .map(|error| format!("Cannot save {}: {error}", path.display()));
    }
}

#[cfg(test)]
mod help_tests;
