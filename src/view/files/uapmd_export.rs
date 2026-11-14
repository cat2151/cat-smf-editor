//! Save the prepared performance through UAPMD's project writer.
use crate::{
    app::App,
    play::{playback::Mode, transport},
};
use cat_smf_editor::{
    model::{Event, Track},
    score::audition_timing::TempoMap,
    uapmd_project::export::{self, Layer, Part},
};

pub(crate) fn dialog(app: &mut App) {
    if let Err(error) = app.remember_loaded() {
        app.load_error = Some(error);
        return;
    }
    let mut dialog = rfd::FileDialog::new()
        .add_filter("UAPMD project", &["uapmdz"])
        .set_file_name("untitled.uapmdz");
    if let Ok(default) = super::uapmd_project_path()
        && let Some(directory) = default.parent()
    {
        dialog = dialog.set_directory(directory);
    }
    let Some(path) = dialog.save_file() else {
        return;
    };
    if app.saved.path().is_some_and(|source| {
        dunce::canonicalize(source)
            .ok()
            .zip(dunce::canonicalize(&path).ok())
            .is_some_and(|(source, output)| source == output)
    }) {
        app.load_error = Some("Projectの出力先には編集用SMFと別のファイルを指定します".into());
        return;
    }
    if let Err(error) = bytes(app).and_then(|bytes| export::write(&path, &bytes)) {
        app.load_error = Some(error);
    }
}

fn finish(mut events: Vec<Event>, end: u64) -> Result<Track, String> {
    events.push(Event {
        tick: end,
        delta: 0,
        status: 0xff,
        meta_type: 0x2f,
        data: vec![],
    });
    events.sort_by_key(|e| e.tick);
    let mut track = Track { events };
    cat_smf_editor::edit::refresh_deltas(&mut track)?;
    Ok(track)
}

pub(super) fn bytes(app: &App) -> Result<Vec<u8>, String> {
    let loaded = app
        .loaded
        .as_ref()
        .ok_or(cat_smf_editor::message::NO_SMF_LOADED)?;
    let music = &loaded.music;
    let collection = cat_smf_editor::score::scene::read(music)?;
    let prepared = if !collection.song.is_empty() {
        crate::play::arrangement::prepare(app, None)?
    } else {
        transport::prepare(app, Mode::AllTracks).ok_or("Could not prepare project performance")?
    };
    let tempo = TempoMap::new(music, 0);
    // A PPQN timeline keeps the source's tempo map. SMPTE performances use a
    // constant 120 BPM timeline; their absolute event times are preserved.
    let division = if music.division & 0x8000 != 0 {
        30000
    } else {
        music.division
    };
    let output_tempo = if music.division & 0x8000 != 0 {
        TempoMap::constant(division, 120.0)?
    } else {
        tempo
    };
    let tick = |seconds: f64| -> Result<u64, String> {
        let t = output_tempo.fractional_tick(seconds);
        if !seconds.is_finite() || seconds < 0.0 || !t.is_finite() || t > u64::MAX as f64 {
            return Err("Invalid project performance time".into());
        }
        Ok(t.round() as u64)
    };
    let end = tick(prepared.active.end)?;
    let master_events = if music.division & 0x8000 == 0 {
        let mut events: Vec<_> = music
            .tracks
            .iter()
            .enumerate()
            .filter(|(index, _)| music.format != 2 || *index == 0)
            .flat_map(|(_, t)| &t.events)
            .filter(|e| e.status == 0xff && matches!(e.meta_type, 0x51 | 0x58) && e.tick <= end)
            .cloned()
            .collect();
        events.sort_by_key(|e| e.tick);
        events
    } else {
        vec![]
    };
    let master = finish(master_events, end)?;
    let mut parts = vec![];
    for (owner, patch, sequence) in prepared.parts {
        if sequence.events.is_empty() {
            continue;
        }
        let solo = patch.layers.iter().any(|l| l.settings.solo);
        let volume = f64::from(crate::play::track_volume::read(&music.tracks[owner])?);
        let mut layers = vec![];
        for layer in &patch.layers {
            let settings = &layer.settings;
            let plugin = match &layer.instrument {
                Some(state) => state.plugin.clone(),
                None => cat_smf_editor::chain::default_instrument()?,
            };
            let mut events = vec![];
            for e in &sequence.events {
                let status = (e.word >> 16) as u8;
                let mut key = ((e.word >> 8) & 127) as u8;
                if matches!(status & 0xf0, 0x80 | 0x90) {
                    let routed = if let Some(kit) = &patch.kit {
                        (kit.layer == settings.id && !kit.muted.contains(&key)).then_some(key)
                    } else if let Some(drums) = &patch.drums {
                        drums.pitch(settings.id, key)
                    } else {
                        settings.pitch(key)
                    };
                    let Some(pitch) = routed else {
                        continue;
                    };
                    key = pitch;
                }
                events.push(Event {
                    tick: tick(e.time - sequence.origin)?,
                    delta: 0,
                    status,
                    meta_type: 0,
                    data: vec![key, (e.word & 127) as u8],
                });
            }
            layers.push(Layer {
                track: finish(events, end)?,
                plugin,
                state: layer.instrument.clone(),
                volume: f64::from(settings.gain()?),
                muted: settings.mute || (solo && !settings.solo),
            });
        }
        if patch.effects.is_empty() {
            parts.extend(layers.into_iter().map(|layer| Part {
                layers: vec![layer],
                effects: vec![],
                volume,
            }));
        } else {
            parts.push(Part {
                layers,
                effects: patch.effects,
                volume,
            });
        }
    }
    export::archive(division, master, parts, &app.audition.master().effects)
}

#[cfg(test)]
mod effects_tests;
#[cfg(test)]
mod tests;
