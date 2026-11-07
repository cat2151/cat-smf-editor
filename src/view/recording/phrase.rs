//! Phrase loop: the measure played from the first key becomes its container's Loop pattern,
//! which repeats until the switch turns it off or the next key plays another over it. Every
//! container repeats its own phrase, so the phrases of the others go on under the keys. A drum
//! container's phrase goes on under its own keys too, and the next replaces only the pitches it
//! starts.
use super::*;
use std::time::Duration;

#[derive(Debug, Default)]
pub(super) enum Looper {
    #[default]
    Off,
    /// On; the next key starts the phrase.
    Waiting,
    Capturing {
        origin: Duration,
        notes: Vec<(Duration, u8, u8)>,
        /// The drum phrase repeating under the capture.
        under: Option<Under>,
    },
    /// The Loop pattern repeating, its notes as offsets from the measure start of each pitch
    /// and velocity (0 for Note Off) at `tempo`.
    Looping {
        pattern: u64,
        notes: Vec<(Duration, u8, u8)>,
        tempo: u32,
        /// Session time of the first repeat.
        start: Duration,
    },
}

#[derive(Debug)]
pub(super) struct Under {
    pattern: u64,
    notes: Vec<(Duration, u8, u8)>,
    tempo: u32,
    start: Duration,
}

/// The playing container's phrase repeats.
pub(super) fn looping(s: &Session) -> bool {
    matches!(s.sound().looper, Looper::Looping { .. })
}

pub(super) fn any_looping(s: &Session) -> bool {
    s.sounds
        .iter()
        .any(|x| matches!(x.looper, Looper::Looping { .. }))
}

fn measure(s: &Session) -> Duration {
    Duration::from_micros(u64::from(s.settings().tempo) * 4)
}

/// Releases what the phrase of container `i` left sounding; the keys keep theirs until
/// released.
fn silence(s: &mut Session, i: usize) {
    let live = &s.sounds[i].live;
    let left = live.phrase.stop();
    if s.click.is_some() {
        live::push(&live.queue, &left, 0);
    }
}

/// The switch of the playing container: on waits for a key, off stops its phrase. The app
/// reopens the keyboard with the switches on when one was on at closing.
pub(super) fn toggle(a: &mut App, s: &mut Session) {
    let i = s.selected;
    s.sounds[i].looper = match s.sounds[i].looper {
        Looper::Off => Looper::Waiting,
        _ => {
            silence(s, i);
            Looper::Off
        }
    };
    let on = s.sounds.iter().any(|x| !matches!(x.looper, Looper::Off));
    a.config.select_phrase_loop(on);
}

/// How far the capture of the playing container has come through its measure, from 0 to 1.
pub(super) fn progress(s: &Session) -> Option<f32> {
    let Looper::Capturing { origin, .. } = &s.sound().looper else {
        return None;
    };
    let done = s.clock.elapsed().saturating_sub(*origin);
    Some((done.as_secs_f32() / measure(s).as_secs_f32()).min(1.0))
}

/// Drops the phrase of container `i`; its switch stays on.
fn restart(s: &mut Session, i: usize) {
    silence(s, i);
    if !matches!(s.sounds[i].looper, Looper::Off) {
        s.sounds[i].looper = Looper::Waiting;
    }
}

/// Where a phrase begun at `at` starts: the bar line before it of another phrase repeating at
/// the same tempo, or the next bar line for a key a sixteenth or less early; `at` without one.
fn origin(s: &Session, at: Duration) -> Duration {
    let length = measure(s);
    let tempo = s.settings().tempo;
    let grid = s.sounds.iter().find_map(|x| match &x.looper {
        Looper::Looping {
            tempo: t, start, ..
        }
        | Looper::Capturing {
            under: Some(Under {
                tempo: t, start, ..
            }),
            ..
        } if *t == tempo => Some(*start),
        _ => None,
    });
    let Some(lap) = grid.and_then(|start| at.checked_sub(start)) else {
        return at;
    };
    let into = Duration::from_nanos((lap.as_nanos() % length.as_nanos().max(1)) as u64);
    let bar = at - into;
    if length - into <= length / 16 {
        bar + length
    } else {
        bar
    }
}

/// Notes that started at `velocity`, or stopped for 0, by the keys; a key over the repeating
/// phrase replaces it with a new phrase from that key, which a drum phrase plays under from the
/// end of the capture.
pub(super) fn heard(s: &mut Session, pitches: &[u8], velocity: u8, at: Duration) {
    if pitches.is_empty() {
        return;
    }
    let on = velocity > 0;
    let i = s.selected;
    let tempo = s.settings().tempo;
    if on && looping(s) {
        let drums = s.settings().drums;
        match std::mem::take(&mut s.sounds[i].looper) {
            Looper::Looping {
                pattern,
                notes,
                tempo: t,
                start,
            } if drums && t == tempo => {
                // On the bar lines of the phrase it goes on under.
                s.sounds[i].looper = Looper::Capturing {
                    origin: start,
                    notes: vec![],
                    under: Some(Under {
                        pattern,
                        notes,
                        tempo,
                        start,
                    }),
                };
                let at = origin(s, at);
                if let Looper::Capturing { origin, .. } = &mut s.sounds[i].looper {
                    *origin = at;
                }
            }
            _ => {
                silence(s, i);
                s.sounds[i].looper = Looper::Waiting;
            }
        }
    }
    if on && matches!(s.sounds[i].looper, Looper::Waiting) {
        s.sounds[i].looper = Looper::Capturing {
            origin: origin(s, at),
            notes: vec![],
            under: None,
        };
    }
    let length = measure(s);
    let sound = &mut s.sounds[i];
    if let Looper::Capturing {
        origin,
        notes,
        under,
    } = &mut sound.looper
    {
        let offset = at.saturating_sub(*origin);
        if offset < length {
            notes.extend(pitches.iter().map(|&p| (offset, p, velocity)));
            if let Some(u) = under {
                let next = layered(&u.notes, &phrase(notes, length));
                let end = *origin + length - u.start;
                sound.live.phrase.replace_from(
                    end,
                    super::kit::swung(&sound.patch, s.division, tempo, &next),
                );
            }
        }
    }
}

/// The captured `notes` as a measure: a key held from before the phrase ends there without
/// having started in it, and one still held at its end stops there.
fn phrase(notes: &[(Duration, u8, u8)], length: Duration) -> Vec<(Duration, u8, u8)> {
    let mut on = [false; 128];
    let mut phrase = vec![];
    for &(offset, pitch, velocity) in notes {
        let sounding = velocity > 0;
        if sounding || on[usize::from(pitch)] {
            on[usize::from(pitch)] = sounding;
            phrase.push((offset, pitch, velocity));
        }
    }
    phrase.extend(
        (0..128u8)
            .filter(|&p| on[usize::from(p)])
            .map(|p| (length, p, 0)),
    );
    phrase
}

/// `over` with the notes of `under` whose pitches it does not start.
fn layered(under: &[(Duration, u8, u8)], over: &[(Duration, u8, u8)]) -> Vec<(Duration, u8, u8)> {
    let mut started = [false; 128];
    for &(_, pitch, _) in over.iter().filter(|n| n.2 > 0) {
        started[usize::from(pitch & 0x7f)] = true;
    }
    let kept = under.iter().filter(|n| !started[usize::from(n.1 & 0x7f)]);
    over.iter().chain(kept).copied().collect()
}

/// Once a measure has passed since the first key, it becomes the container's Loop pattern and
/// the audio thread repeats that pattern; the playing container's is shown behind the keyboard.
pub(super) fn follow(a: &mut App, s: &mut Session) {
    for i in 0..s.sounds.len() {
        follow_capture(a, s, i);
    }
}

fn follow_capture(a: &mut App, s: &mut Session, i: usize) {
    let length = measure(s);
    let Looper::Capturing { origin, notes, .. } = &s.sounds[i].looper else {
        return;
    };
    if s.clock.elapsed() < *origin + length {
        return;
    }
    let phrase = phrase(notes, length);
    let controls = s
        .settings()
        .modulation
        .changes(s.sweep_start(), *origin, *origin + length)
        .into_iter()
        .map(|(at, value)| (at - *origin, value))
        .collect::<Vec<_>>();
    let end = *origin + length;
    let tempo = s.settings().tempo;
    let quantize = s.settings().quantize;
    let Looper::Capturing { under, .. } =
        std::mem::replace(&mut s.sounds[i].looper, Looper::Waiting)
    else {
        unreachable!()
    };
    let keep = under.is_some();
    let written = super::take::source(a, s.sounds[i].container).and_then(|source| {
        let mut id = 0;
        a.edit_tracks(|music| {
            id = cat_smf_editor::score::pattern::recording::repeat(
                music, source, tempo, &phrase, &controls, keep, quantize,
            )?;
            Ok(())
        })?;
        super::take::adopt(a, s, id)?;
        Ok(id)
    });
    let pattern = match written {
        Ok(id) => id,
        Err(error) => {
            s.error = Some(error);
            silence(s, i);
            return;
        }
    };
    let Some(notes) = pattern_notes(a, pattern, tempo) else {
        silence(s, i);
        return;
    };
    let audible = super::kit::swung(&s.sounds[i].patch, s.division, tempo, &notes);
    let phrase = &s.sounds[i].live.phrase;
    let start = match under {
        // From the lap that began at the capture's end, unless that has already started.
        Some(u) => {
            phrase.replace_from(end - u.start, audible);
            u.start
        }
        None => {
            phrase.play(s.clock + end, length, audible);
            end
        }
    };
    s.sounds[i].looper = Looper::Looping {
        pattern,
        notes,
        tempo,
        start,
    };
    if i == s.selected {
        reveal(a, s);
    }
}

/// Ticks at `division` into Loop pattern `id` that a repeating phrase plays now.
pub(super) fn tick(s: &Session, id: u64, division: u16) -> Option<u64> {
    let at = s.clock.elapsed();
    s.sounds.iter().find_map(|x| {
        let (tempo, start) = match &x.looper {
            Looper::Looping {
                pattern,
                tempo,
                start,
                ..
            }
            | Looper::Capturing {
                under:
                    Some(Under {
                        pattern,
                        tempo,
                        start,
                        ..
                    }),
                ..
            } if *pattern == id => (u128::from(*tempo).max(1), *start),
            _ => return None,
        };
        let into = at.checked_sub(start)?.as_micros() % (tempo * 4);
        u64::try_from(into * u128::from(division) / tempo).ok()
    })
}

fn pattern_notes(a: &App, id: u64, tempo: u32) -> Option<Vec<(Duration, u8, u8)>> {
    let music = &a.loaded.as_ref()?.music;
    cat_smf_editor::score::pattern::recording::looped(music, id, tempo).ok()
}

/// The event list and the Song window show the playing container's repeating Loop pattern.
pub(super) fn reveal(a: &mut App, s: &mut Session) {
    let Looper::Looping { pattern, .. } = s.sound().looper else {
        return;
    };
    if let Err(error) = show(a, pattern) {
        s.error = Some(error);
    }
}

/// The event list and the Song window show the Loop pattern, as a take is shown.
fn show(a: &mut App, id: u64) -> Result<(), String> {
    super::super::patterns::library::edit(a, id)?;
    let song = a.patterns.workspace.selection_mut(Pane::Song);
    song.choice = Some(PatternOrScene::Pattern(id));
    song.query.clear();
    song.reveal_choice = true;
    Ok(())
}

/// The velocity of the playing container's repeat's latest Note On at `at`, session time.
pub(super) fn replayed(s: &Session, at: Duration) -> Option<u8> {
    let Looper::Looping {
        notes,
        tempo,
        start,
        ..
    } = &s.sound().looper
    else {
        return None;
    };
    let lap = at.checked_sub(*start)?;
    let length = Duration::from_micros(u64::from(*tempo) * 4);
    let position = Duration::from_nanos((lap.as_nanos() % length.as_nanos().max(1)) as u64);
    let on = notes.iter().filter(|n| n.2 > 0);
    on.clone()
        .rfind(|n| n.0 <= position)
        .or_else(|| on.clone().next_back())
        .map(|n| n.2)
}

mod song;
pub(super) use song::to_song;

mod sounds;
pub(super) use sounds::{adopt, browse, display, follow_patch, retune};
