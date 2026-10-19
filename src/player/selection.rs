use crate::app::App;
use cat_smf_editor::score::instrument::{self as chain_settings, Patch as Settings};
use std::path::{Path, PathBuf};

pub const DOWNLOAD: &str = "https://github.com/cat2151/cat-plugin-player/releases";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Browser {
    Patch,
    Effect,
    MasterEffect,
}

#[derive(Default)]
pub struct State {
    pub setup: crate::player::setup::Setup,
    pub layer: Option<u64>,
    pub open: bool,
    pub error: Option<String>,
    pub executable: Option<PathBuf>,
    pub no_notes_error: bool,
    #[cfg(windows)]
    pub resident: Option<crate::player::resident_link::Link>,
    /// Starts the player; `None` runs the executable. Tests connect a client instead.
    #[cfg(windows)]
    pub launch: Option<crate::player::resident_link::Launch>,
}

pub fn resolve(
    explicit: Option<PathBuf>,
    executable: &Path,
    path: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return path
            .is_file()
            .then_some(path)
            .ok_or("--player does not name an executable file".into());
    }
    if let Some(parent) = executable.parent() {
        let candidate = parent.join("cat-plugin-player.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    if let Some(candidate) = sibling_release(executable)
        && candidate.is_file()
    {
        return Ok(candidate);
    }
    if let Some(path) = path {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join("cat-plugin-player.exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err("cat-plugin-player is not installed. Build the sibling cat-plugin-player repo in release mode, place its executable beside the editor, add it to PATH, or pass --player <path>.".into())
}

fn sibling_release(executable: &Path) -> Option<PathBuf> {
    let profile = executable.parent()?;
    if !matches!(profile.file_name()?.to_str()?, "debug" | "release") {
        return None;
    }
    let target = profile.parent()?;
    if target.file_name()? != "target" {
        return None;
    }
    Some(
        target
            .parent()?
            .parent()?
            .join("cat-plugin-player/target/release/cat-plugin-player.exe"),
    )
}

#[cfg(test)]
pub fn open(app: &mut App, browser: Browser) {
    app.player_selection.layer = None;
    open_mode(app, browser, false, None);
}

pub fn open_layer(app: &mut App, id: u64) {
    app.player_selection.layer = Some(id);
    open_mode(app, Browser::Patch, false, None);
}

/// Opens the effect browser on chain position `slot`; the position after the
/// last effect adds one.
pub fn open_slot(app: &mut App, browser: Browser, slot: usize) {
    open_mode(app, browser, false, Some(slot));
}

pub(crate) fn open_mode(app: &mut App, browser: Browser, bulk: bool, slot: Option<usize>) {
    app.player_selection.no_notes_error = false;
    let Some(loaded) = &app.loaded else {
        return;
    };
    if browser == Browser::Patch
        && !bulk
        && !crate::player::bulk::has_notes(&loaded.music.tracks[app.track])
    {
        app.player_selection.open = true;
        app.player_selection.no_notes_error = true;
        app.player_selection.error = Some("このtrackにはノートがありません（conductor track）。音色指定するtrackを選択してください。".into());
        return;
    }
    if browser == Browser::MasterEffect && loaded.music.format != 1 {
        app.player_selection.open = true;
        app.player_selection.error = Some("Master effects editing requires SMF format 1".into());
        return;
    }
    let track = if bulk {
        crate::player::bulk::targets(&loaded.music)[0]
    } else {
        app.track
    };
    #[cfg(windows)]
    let result = crate::player::resident_link::request(
        app,
        crate::player::resident_link::Request {
            browser,
            track,
            slot,
            bulk,
        },
    );
    #[cfg(not(windows))]
    let result: Result<(), String> = {
        let _ = (track, slot);
        Err("The resident player requires Windows".into())
    };
    match result {
        Ok(()) => {
            app.player_selection.error = None;
            app.player_selection.open = false;
        }
        Err(error) => {
            app.player_selection.open = true;
            app.player_selection.error = Some(error);
        }
    }
}

/// Writes master effects into the loaded SMF, keeping the playback position.
pub fn store_master(
    app: &mut App,
    master: cat_smf_editor::master_effects::Settings,
) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if *app.audition.master() == master {
        return Ok(());
    }
    let session = app.session();
    let before = loaded.music.clone();
    let mut music = before.clone();
    cat_smf_editor::master_references::write(
        &mut music,
        &cat_smf_editor::master_references::Effects {
            effects: master
                .effects
                .iter()
                .map(cat_smf_editor::score::layer_track::effect_reference)
                .collect::<Result<_, _>>()?,
        },
        &mut app
            .patch_roots
            .lookup(app.player_selection.executable.clone()),
    )?;
    app.saved.remember(&loaded.original, &music)?;
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music = music;
    app.undo.push(before);
    loaded.refresh();
    app.audition.set_master(master);
    app.generation += 1;
    if let Some(session) = session {
        app.restore_position(&session);
    }
    Ok(())
}

/// Writes one track's chain settings into the loaded SMF, keeping the playback position.
pub fn store_track(app: &mut App, track: usize, settings: &Settings) -> Result<(), String> {
    let loaded = app.loaded.as_ref().ok_or("No SMF is loaded")?;
    if loaded.patches[track] == *settings {
        return Ok(());
    }
    let session = app.session();
    let before = loaded.music.clone();
    let mut music = before.clone();
    let roots = &mut app
        .patch_roots
        .lookup(app.player_selection.executable.clone());
    chain_settings::write(&mut music.tracks[track], settings, roots)?;
    app.saved.remember(&loaded.original, &music)?;
    // Playback continues through a bypass change so that it can be compared by ear.
    let live = loaded.patches[track].same_except_bypass(settings);
    let resume = if live {
        app.audition.set_effect_bypass(track, settings);
        None
    } else {
        let resume = crate::play::selection_playback::suspend(app);
        app.audition.stop();
        resume
    };
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music = music;
    app.undo.push(before);
    loaded.patches[track] = settings.clone();
    loaded.refresh();
    app.generation += 1;
    if let Some(session) = session {
        app.restore_position(&session);
    }
    if let Some(resume) = resume {
        crate::play::selection_playback::restore(app, resume);
    }
    Ok(())
}

#[path = "selection_ui.rs"]
mod ui;
pub use ui::show;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod bulk_tests;

#[cfg(test)]
mod master_tests;
