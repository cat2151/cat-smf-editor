//! Atomic kit confirmation; browser movement never edits the song.
use crate::app::App;
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::Kit,
    resident_ipc::Open,
    score::instrument::{Patch, ResolvedLayer},
};

pub(crate) fn open(app: &mut App, create: bool, pattern: bool) {
    let Some(loaded) = &app.loaded else {
        return;
    };
    let layer = if create {
        None
    } else {
        let Some(kit) = &loaded.patches[app.track].kit else {
            return;
        };
        Some(kit.layer)
    };
    app.player_selection.layer = layer;
    app.player_selection.kit_pattern = pattern;
    app.player_selection.kit_request += 1;
    app.player_selection.kit_open = true;
    super::selection::open_mode(app, super::selection::Browser::Kit, false, None);
}

pub(crate) fn apply(
    app: &mut App,
    target: Open,
    slot: PluginState,
    kit: Option<Kit>,
) -> Result<(), String> {
    let Open::Kit {
        track,
        layer,
        song,
        request,
    } = target
    else {
        return Err("Expected kit target".into());
    };
    if song != app.song_id || track != app.track || request != app.player_selection.kit_request {
        return Err("Kit selection target changed".into());
    }
    let mut kit = kit.ok_or("Kit metadata is missing")?;
    kit.validate()?;
    if let Some(id) = layer {
        let mut patch = app.loaded.as_ref().ok_or("No song")?.patches[track].clone();
        let old = patch
            .kit
            .as_ref()
            .filter(|k| k.layer == id)
            .ok_or("Kit was removed")?;
        kit.rows = old.rows.clone();
        kit.muted = old.muted.clone();
        kit.swing = old.swing.clone();
        for note in &mut kit.notes {
            note.velocity = old.velocity(note.note);
        }
        for old_note in &old.notes {
            if !kit.notes.iter().any(|n| n.note == old_note.note)
                && app.loaded.as_ref().unwrap().music.tracks[track]
                    .events
                    .iter()
                    .any(|e| {
                        cat_smf_editor::score::note_view::note_on(e) && e.data[0] == old_note.note
                    })
            {
                kit.notes.push(old_note.clone());
            }
        }
        kit.notes.sort_by_key(|n| n.note);
        app.audition.stop_key();
        patch
            .layers
            .iter_mut()
            .find(|l| l.settings.id == id)
            .ok_or("Kit layer was removed")?
            .instrument = Some(slot);
        patch.kit = Some(kit);
        super::selection::store_track(app, track, &patch)
    } else {
        let patch = Patch {
            kit: Some(kit),
            layers: vec![ResolvedLayer {
                settings: Default::default(),
                instrument: Some(slot),
            }],
            ..Default::default()
        };
        patch.validate()?;
        if app.player_selection.kit_pattern {
            crate::view::patterns::library::add_container_with(app, Some(patch))?;
            crate::view::drums::display(app);
        } else {
            let mut music = app.loaded.as_ref().ok_or("No song")?.music.clone();
            let mut source = cat_smf_editor::model::Track { events: vec![] };
            cat_smf_editor::score::instrument::write(
                &mut source,
                &patch,
                &mut app
                    .patch_roots
                    .lookup(app.player_selection.executable.clone()),
            )?;
            music.tracks.push(source);
            music.format = 1;
            let track = music.tracks.len() - 1;
            app.edit_tracks(|m| {
                *m = music;
                Ok(())
            })?;
            app.select_track(track);
            crate::view::drums::display(app);
        }
        Ok(())
    }
}
