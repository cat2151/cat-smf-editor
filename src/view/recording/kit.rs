//! Kit rows share the editor's assignments; pages are session state, never another mapping.
use super::*;
use cat_smf_editor::score::drum_kit;

fn rows(a: &App, s: &Session) -> Vec<Option<u8>> {
    let sound = s.sound();
    let Some(kit) = &sound.patch.kit else {
        return vec![];
    };
    let Some(track) = a
        .loaded
        .as_ref()
        .and_then(|l| l.music.tracks.get(sound.track))
    else {
        return vec![];
    };
    drum_kit::layout::rows(track, kit)
        .into_iter()
        .rev()
        .collect()
}

pub(super) fn sync(a: &App, s: &mut Session) {
    let kit = s.sound().patch.kit.is_some();
    let rows = rows(a, s);
    let bank = s.sound().kit_bank.min(rows.len().saturating_sub(1) / 48);
    s.sounds[s.selected].kit_bank = bank;
    if let Some(r) = s.recorder.as_mut() {
        if kit {
            r.settings_mut().drums = true;
        }
        if kit && !recording::Mode::KIT.contains(&r.settings().mode) {
            r.settings_mut().mode = recording::Mode::Note;
        }
        r.set_kit_chromatic(s.sounds[s.selected].patch.kit.as_ref().map(|kit| {
            let mut available = [false; 128];
            for note in &kit.notes {
                if note.note < 128 {
                    available[usize::from(note.note)] = true;
                }
            }
            (s.sounds[s.selected].kit_base, available)
        }));
        r.set_kit_rows(
            kit.then(|| std::array::from_fn(|i| rows.get(bank * 48 + i).copied().flatten())),
        );
    }
}

pub(super) fn page(a: &App, s: &mut Session, up: bool) {
    if s.settings().mode == recording::Mode::Chromatic {
        let base = &mut s.sounds[s.selected].kit_base;
        *base = if up {
            base.saturating_add(12).min(127)
        } else {
            base.saturating_sub(12)
        };
        sync(a, s);
        return;
    }
    let last = rows(a, s).len().saturating_sub(1) / 48;
    let bank = &mut s.sounds[s.selected].kit_bank;
    *bank = if up {
        (*bank + 1).min(last)
    } else {
        bank.saturating_sub(1)
    };
    sync(a, s);
}

pub(super) fn pitches(s: &Session, pad: recording::Pad) -> Vec<u8> {
    s.recorder
        .as_ref()
        .map_or_else(|| recording::pitches(s.settings(), pad), |r| r.pitches(pad))
}

pub(super) fn caption(s: &Session, pad: recording::Pad) -> (String, bool) {
    let Some(kit) = &s.sound().patch.kit else {
        return window::caption(s.settings(), pad);
    };
    let Some(pitch) = pitches(s, pad).first().copied() else {
        return (String::new(), false);
    };
    let name = kit
        .notes
        .iter()
        .find(|n| n.note == pitch)
        .map_or("", |n| n.name.as_str());
    (
        if name.is_empty() {
            pitch.to_string()
        } else {
            name.to_owned()
        },
        false,
    )
}

/// Reuse ordinary playback's timing transform; stored notes and capture offsets stay straight.
pub(super) fn swung(
    patch: &Patch,
    division: u16,
    tempo: u32,
    notes: &[(std::time::Duration, u8, u8)],
) -> Vec<(std::time::Duration, u8, u8)> {
    use crate::play::audio_sequence::{MidiEvent, Sequence};
    use cat_smf_editor::score::audition_timing::TempoMap;
    if patch.kit.is_none() {
        return notes.to_vec();
    }
    let map = TempoMap::constant(division, 60_000_000.0 / f64::from(tempo))
        .expect("validated recording tempo");
    let sequence = Sequence {
        notes: vec![],
        events: notes
            .iter()
            .map(|&(at, pitch, velocity)| MidiEvent {
                time: at.as_secs_f64(),
                word: live::word(pitch, velocity),
            })
            .collect(),
        origin: 0.0,
        end: f64::from(tempo) * 4.0 / 1_000_000.0,
    }
    .kit_swing(patch.kit.as_ref(), division, &map);
    sequence
        .events
        .into_iter()
        .map(|e| {
            (
                std::time::Duration::from_secs_f64(e.time),
                ((e.word >> 8) & 127) as u8,
                (e.word & 127) as u8,
            )
        })
        .collect()
}
