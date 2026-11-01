//! Phrase loop: the measure played from the first key repeats until the switch turns it off
//! or the next key starts another.
use super::*;
use crate::play::audition::live::Live;
use std::time::Duration;

#[derive(Debug, Default)]
pub(super) enum Looper {
    #[default]
    Off,
    /// On; the next key starts the phrase.
    Waiting,
    Capturing {
        origin: Duration,
        notes: Vec<(Duration, u8, bool)>,
    },
    Looping,
}

pub(super) fn looping(s: &Session) -> bool {
    matches!(s.looper, Looper::Looping)
}

fn measure(s: &Session) -> Duration {
    Duration::from_micros(u64::from(s.settings().tempo) * 4)
}

/// Releases what the phrase left sounding; the keys keep theirs until released.
pub(super) fn silence(s: &mut Session) {
    let left = s.phrase.as_ref().map_or(vec![], |p| p.stop());
    if let Some(q) = &s.queue {
        for pitch in left {
            live::push(q, pitch, false);
        }
    }
}

/// The switch: on waits for a key, off stops the phrase. The app reopens the keyboard with
/// the switch on when it closed so.
pub(super) fn toggle(a: &mut App, s: &mut Session) {
    s.looper = match s.looper {
        Looper::Off => Looper::Waiting,
        _ => {
            silence(s);
            Looper::Off
        }
    };
    a.config
        .select_phrase_loop(!matches!(s.looper, Looper::Off));
}

/// How far the capture has come through its measure, from 0 to 1.
pub(super) fn progress(s: &Session) -> Option<f32> {
    let Looper::Capturing { origin, .. } = &s.looper else {
        return None;
    };
    let done = s.clock.elapsed().saturating_sub(*origin);
    Some((done.as_secs_f32() / measure(s).as_secs_f32()).min(1.0))
}

/// Another instrument drops the phrase; the switch stays on.
pub(super) fn restart(s: &mut Session) {
    silence(s);
    if !matches!(s.looper, Looper::Off) {
        s.looper = Looper::Waiting;
    }
}

/// Notes that started or stopped sounding by the keys; a key over the repeating phrase
/// replaces it with a new phrase from that key.
pub(super) fn heard(s: &mut Session, pitches: &[u8], on: bool, at: Duration) {
    if pitches.is_empty() {
        return;
    }
    if on && looping(s) {
        silence(s);
        s.looper = Looper::Waiting;
    }
    if on && matches!(s.looper, Looper::Waiting) {
        s.looper = Looper::Capturing {
            origin: at,
            notes: vec![],
        };
    }
    let length = measure(s);
    if let Looper::Capturing { origin, notes } = &mut s.looper {
        let offset = at.saturating_sub(*origin);
        if offset < length {
            notes.extend(pitches.iter().map(|&p| (offset, p, on)));
        }
    }
}

/// Once a measure has passed since the first key, the audio thread repeats it.
pub(super) fn follow(s: &mut Session) {
    let length = measure(s);
    let Looper::Capturing { origin, notes } = &s.looper else {
        return;
    };
    if s.clock.elapsed() < *origin + length {
        return;
    }
    let mut on = [false; 128];
    let mut phrase = vec![];
    for &(offset, pitch, sounding) in notes {
        // A key held from before the phrase ends there without having started in it.
        if sounding || on[usize::from(pitch)] {
            on[usize::from(pitch)] = sounding;
            phrase.push((offset, pitch, sounding));
        }
    }
    phrase.extend(
        (0..128u8)
            .filter(|&p| on[usize::from(p)])
            .map(|p| (length, p, false)),
    );
    if let Some(p) = &s.phrase {
        p.play(s.clock + *origin + length, length, phrase);
    }
    s.looper = Looper::Looping;
}

/// Follows patch edits of the playing container without stopping the phrase.
pub(super) fn follow_patch(a: &mut App, s: &mut Session) {
    if s.generation == a.generation {
        return;
    }
    s.generation = a.generation;
    let Some(loaded) = a.loaded.as_ref() else {
        return;
    };
    let mut changed = false;
    for (i, sound) in s.sounds.iter_mut().enumerate() {
        if let Some(patch) = sound
            .container
            .and_then(|_| loaded.patches.get(sound.track))
            && *patch != sound.patch
        {
            sound.patch = patch.clone();
            changed |= i == s.selected;
        }
    }
    let (Some(queue), Some(click), Some(phrase)) = (&s.queue, &s.click, &s.phrase) else {
        return;
    };
    let live = Live {
        queue: queue.clone(),
        click: click.clone(),
        phrase: phrase.clone(),
    };
    let sound = &s.sounds[s.selected];
    if changed && !a.audition.retune_live(sound.track, &sound.patch, &live) {
        s.error = Some(
            a.audition
                .error()
                .unwrap_or(message::CANNOT_OPEN_RECORDING_INSTRUMENT)
                .to_owned(),
        );
    }
}

/// What the panel shows: the patch name, or the layer count of a layered container.
pub(super) fn display(s: &Session) -> (String, bool) {
    let layers = &s.sounds[s.selected].patch.layers;
    if layers.len() > 1 {
        return (format!("{} layers", layers.len()), true);
    }
    let label = s.sounds[s.selected].patch.label();
    let stem = std::path::Path::new(label)
        .file_stem()
        .map_or(label.into(), |s| s.to_string_lossy().into_owned());
    (stem, false)
}

/// Opens the patch browser, or the layer editor of a layered container, on the playing
/// container as the track's own patch command does.
pub(super) fn browse(a: &mut App, ctx: &egui::Context, s: &Session) {
    let sound = &s.sounds[s.selected];
    if !looping(s) || sound.container.is_none() {
        return;
    }
    a.select_track(sound.track);
    if display(s).1 {
        super::super::layers::open(a);
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
    } else {
        crate::player::bulk::request(a);
    }
}
