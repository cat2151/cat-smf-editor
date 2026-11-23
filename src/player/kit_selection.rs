//! Atomic kit confirmation; browser movement never edits the song.
use crate::app::App;
use cat_smf_editor::{
    chain_settings::PluginState,
    drum_kit::Kit,
    resident_ipc::Open,
    score::drum_kit::{kick_pitch, GM_KICK},
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
        let loaded = app.loaded.as_ref().ok_or("No song")?;
        let mut patch = loaded.patches[track].clone();
        let layer = patch
            .layers
            .iter_mut()
            .find(|l| l.settings.id == id)
            .ok_or("Kit layer was removed")?;
        let old = patch
            .kit
            .as_ref()
            .filter(|k| k.layer == id)
            .ok_or("Kit was removed")?;
        kit.rows = old.rows.clone();
        kit.muted = old.muted.clone();
        kit.swing = old.swing.clone();
        let source = &loaded.music.tracks[track];
        let starter = if layer.instrument.is_none() {
            old.notes
                .iter()
                .any(|note| note.note == GM_KICK && note.name == "Kick")
                .then_some(GM_KICK)
        } else {
            #[cfg(windows)]
            if crate::view::song_challenge::awaiting_browser(app, target) {
                kick_pitch(old)
            } else {
                None
            }
            #[cfg(not(windows))]
            None
        };
        let kick = starter.and_then(|from| starter_kick(from, old, &kit, source));
        if let Some((from, to)) = kick {
            if let Some(rows) = &mut kit.rows
                && let Some(row) = rows.iter_mut().find(|row| **row == Some(from))
            {
                *row = Some(to);
            }
            if let Ok(index) = kit.muted.binary_search(&from) {
                kit.muted[index] = to;
                kit.muted.sort_unstable();
                kit.muted.dedup();
            }
            if let Some(swing) = kit.swing.remove(&from) {
                kit.swing.insert(to, swing);
            }
        }
        for note in &mut kit.notes {
            let pitch = kick
                .filter(|&(_, to)| to == note.note)
                .map_or(note.note, |(from, _)| from);
            note.velocity = old.velocity(pitch);
        }
        for old_note in &old.notes {
            if !kick.is_some_and(|(from, _)| old_note.note == from)
                && !kit.notes.iter().any(|n| n.note == old_note.note)
                && source.events.iter().any(|e| {
                    cat_smf_editor::score::note_view::note_on(e) && e.data[0] == old_note.note
                })
            {
                kit.notes.push(old_note.clone());
            }
        }
        kit.notes.sort_by_key(|n| n.note);
        app.audition.stop_key();
        layer.instrument = Some(slot);
        patch.kit = Some(kit);
        super::selection::store_track_with(app, track, &patch, |music| {
            if let Some((from, to)) = kick {
                for event in &mut music.tracks[track].events {
                    if matches!(event.status & 0xf0, 0x80 | 0x90)
                        && event.data.first() == Some(&from)
                    {
                        event.data[0] = to;
                    }
                }
            }
            Ok(())
        })
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

/// Give the starter hit the selected kit's kick pitch without merging existing rows or hits.
fn starter_kick(
    from: u8,
    old: &Kit,
    selected: &Kit,
    source: &cat_smf_editor::model::Track,
) -> Option<(u8, u8)> {
    let to = kick_pitch(selected)?;
    let hit_on = |pitch| {
        source
            .events
            .iter()
            .any(|e| cat_smf_editor::score::note_view::note_on(e) && e.data[0] == pitch)
    };
    (from != to
        && hit_on(from)
        && !hit_on(to)
        && !old.rows.iter().flatten().any(|row| *row == Some(to))
        && !old.muted.contains(&to)
        && !old.swing.contains_key(&to))
    .then_some((from, to))
}
