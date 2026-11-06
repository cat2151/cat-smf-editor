//! Keep the keyboard sounds in sync with container and pattern edits.
use super::*;

/// After an edit, each repeat follows its Loop pattern from its next lap; without the pattern
/// the phrase stops and the switch waits for a key.
fn follow_pattern(a: &App, s: &mut Session) {
    for i in 0..s.sounds.len() {
        let sound = &mut s.sounds[i];
        let Looper::Looping {
            pattern,
            notes,
            tempo,
            ..
        } = &mut sound.looper
        else {
            continue;
        };
        match pattern_notes(a, *pattern, *tempo) {
            Some(now) if now == *notes => {}
            Some(now) => {
                sound.live.phrase.replace(now.clone());
                *notes = now;
            }
            None => restart(s, i),
        }
    }
}

/// Containers new to the song join the keyboard in track order, the first in place of the
/// sound standing in for a song without containers; `true` when any joined.
pub(in crate::view::recording) fn adopt(a: &App, s: &mut Session) -> bool {
    let Some(loaded) = a.loaded.as_ref() else {
        return false;
    };
    let Ok(catalog) = Catalog::read(&loaded.music) else {
        return false;
    };
    let mut new: Vec<_> = catalog
        .tracks
        .iter()
        .filter(|(id, t)| {
            catalog.sources.contains(t)
                && loaded.patches[**t].kit.is_none()
                && !s.sounds.iter().any(|x| x.container == Some(**id))
        })
        .map(|(&id, &track)| (track, id))
        .collect();
    if new.is_empty() {
        return false;
    }
    new.sort();
    for (track, id) in new {
        let sound = Sound {
            container: Some(id),
            track,
            patch: loaded.patches[track].clone(),
            name: cat_smf_editor::score::pattern::container_name(&loaded.music, id, track),
            live: live::Live::default(),
            looper: if a.config.phrase_loop {
                Looper::Waiting
            } else {
                Looper::Off
            },
        };
        match s.sounds.iter_mut().find(|x| x.container.is_none()) {
            Some(standing) => {
                standing.container = sound.container;
                standing.track = sound.track;
                standing.patch = sound.patch;
                standing.name = sound.name;
            }
            None => s.sounds.push(sound),
        }
    }
    true
}

/// Follows edits of the song without stopping the phrases: the Loop patterns, the containers'
/// moves, patches and renames. `true` when a container the keyboard plays is gone.
pub(in crate::view::recording) fn follow_patch(a: &mut App, s: &mut Session) -> bool {
    if s.generation == a.generation {
        return false;
    }
    s.generation = a.generation;
    follow_pattern(a, s);
    let Some(loaded) = a.loaded.as_ref() else {
        return true;
    };
    let Ok(catalog) = Catalog::read(&loaded.music) else {
        return true;
    };
    let mut changed = false;
    for sound in s.sounds.iter_mut() {
        if let Some(id) = sound.container {
            let Some(&track) = catalog.tracks.get(&id) else {
                return true;
            };
            if track != sound.track {
                sound.track = track;
                changed = true;
            }
            sound.name =
                cat_smf_editor::score::pattern::container_name(&loaded.music, id, sound.track);
        } else if sound.track != loaded.music.tracks.len() {
            sound.track = loaded.music.tracks.len();
            changed = true;
        }
        if let Some(patch) = sound
            .container
            .and_then(|_| loaded.patches.get(sound.track))
            && *patch != sound.patch
        {
            sound.patch = patch.clone();
            changed = true;
        }
    }
    if adopt(a, s) || changed {
        retune(a, s);
    }
    false
}

/// The playing instruments take the containers' current sounds.
pub(in crate::view::recording) fn retune(a: &mut App, s: &mut Session) {
    let Some(click) = &s.click else {
        return;
    };
    if !a
        .audition
        .retune_live(&crate::view::recording::play::sounds(s), click)
    {
        s.error = Some(
            a.audition
                .error()
                .unwrap_or(message::CANNOT_OPEN_RECORDING_INSTRUMENT)
                .to_owned(),
        );
    }
}

/// What the panel shows: the patch name, or the layer count of a layered container.
pub(in crate::view::recording) fn display(s: &Session) -> (String, bool) {
    let layers = &s.sound().patch.layers;
    if layers.len() > 1 {
        return (format!("{} layers", layers.len()), true);
    }
    let name = layers.first().map_or("—".into(), |l| l.name());
    (name, false)
}

/// Opens the patch browser, or the layer editor of a layered container, on the playing
/// container as the track's own patch command does.
pub(in crate::view::recording) fn browse(a: &mut App, ctx: &egui::Context, s: &Session) {
    let sound = s.sound();
    if !looping(s) || sound.container.is_none() {
        return;
    }
    a.select_track(sound.track);
    if display(s).1 {
        crate::view::layers::open(a);
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
    } else {
        crate::player::bulk::request(a);
    }
}
