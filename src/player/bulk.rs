use crate::{
    app::App,
    player::selection::{open_mode, Browser},
    store::config::ProgramTarget,
};
use cat_smf_editor::{
    model::{Music, Track},
    score::instrument::{self as chain_settings, Patch as Settings},
    score::note_view::note_on,
    score::program_name,
};
use eframe::egui;

pub(crate) fn has_notes(track: &Track) -> bool {
    track.events.iter().any(note_on)
}

pub(crate) fn targets(music: &Music) -> Vec<usize> {
    music
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| has_notes(track))
        .map(|(index, _)| index)
        .collect()
}

pub(crate) fn same_program(music: &Music) -> bool {
    let tracks = targets(music);
    if tracks.len() < 2 {
        return false;
    }
    let Some(name) = program_name::read(&music.tracks[tracks[0]]) else {
        return false;
    };
    tracks
        .iter()
        .all(|&track| program_name::read(&music.tracks[track]).as_ref() == Some(&name))
}

pub(crate) fn request(app: &mut App) {
    app.player_selection.layer = None;
    let bulk = app.config.program_target == ProgramTarget::Auto
        && app.loaded.as_ref().is_some_and(|l| same_program(&l.music));
    open_mode(app, Browser::Patch, bulk, None);
}

pub(crate) fn accept(app: &mut App, settings: &Settings) -> Result<(), String> {
    let loaded = app
        .loaded
        .as_ref()
        .ok_or("Selection target is no longer loaded")?;
    let tracks = targets(&loaded.music);
    let session = app.session();
    let before = loaded.music.clone();
    let mut music = before.clone();
    let mut changed = false;
    let mut patches = Vec::new();
    let roots = &mut app
        .patch_roots
        .lookup(app.player_selection.executable.clone());
    for &track in &tracks {
        let mut patch = loaded.patches[track].clone();
        let selected = settings.layers.first().ok_or("No selected layer")?;
        if let Some(primary) = patch.layers.first_mut() {
            primary.instrument = selected.instrument.clone();
        } else {
            patch.layers.push(selected.clone());
        }
        patch.validate()?;
        changed |= patch != loaded.patches[track];
        chain_settings::write(&mut music.tracks[track], &patch, roots)?;
        patches.push((track, patch));
    }
    if !changed {
        return Ok(());
    }
    app.saved.remember(&loaded.original, &music)?;
    let resume = crate::play::selection_playback::suspend(app);
    app.audition.stop();
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music = music;
    app.undo.push(before);
    for (track, patch) in patches {
        loaded.patches[track] = patch;
    }
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

pub(crate) fn show_note_error(app: &mut App, ctx: &egui::Context) {
    crate::view::commands::route(app, ctx);
    if !app.player_selection.no_notes_error {
        return;
    }
    let mut close = false;
    egui::Modal::new(egui::Id::new("no-notes-program-error")).show(ctx, |ui| {
        ui.heading("音色を指定できません");
        ui.colored_label(
            egui::Color32::LIGHT_RED,
            app.player_selection
                .error
                .as_deref()
                .unwrap_or("このtrackにはノートがありません。"),
        );
        close |= ui.button("閉じる [Enter / Esc]").clicked();
    });
    if close {
        app.player_selection.no_notes_error = false;
        app.player_selection.open = false;
        app.player_selection.error = None;
    }
}
