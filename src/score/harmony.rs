//! Key, scale, pentatonic and chord at a song tick, from the key signatures and the chord guide.
//! Knows only song ticks: nothing about patterns or how the result is shown.
use crate::{
    model::Music,
    score::{
        chord_guide,
        key_signature::{KeySignature, KeySignatures},
        pattern::recording::{Key, Scale},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A set of the twelve pitch classes, C = bit 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PitchClasses(u16);

impl PitchClasses {
    pub const ALL: Self = Self(0xfff);

    pub fn of(pitches: impl IntoIterator<Item = u8>) -> Self {
        Self(pitches.into_iter().fold(0, |bits, p| bits | 1 << (p % 12)))
    }

    /// `steps` semitones above `tonic`.
    pub fn on(tonic: u8, steps: &[u8]) -> Self {
        Self::of(steps.iter().map(|s| tonic % 12 + s))
    }

    /// Whether the pitch class of `pitch` (any octave) is in the set.
    pub fn contains(self, pitch: u8) -> bool {
        self.0 & 1 << (pitch % 12) != 0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Which pitches a harmony offers, from all twelve down to the chord tones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Set {
    Chromatic,
    Scale,
    Penta,
    #[default]
    Chord,
}

impl Set {
    pub const ALL: [Self; 4] = [Self::Chromatic, Self::Scale, Self::Penta, Self::Chord];

    /// Chromatic, Scale, Penta, Chord, then Chromatic again.
    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|&s| s == self).unwrap();
        Self::ALL[(at + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    pub name: String,
    pub pitches: PitchClasses,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harmony {
    /// `None` where no key signature has taken effect; the sets then follow C major.
    pub key: Option<KeySignature>,
    /// The key's tonic as a pitch class (0 = C).
    pub tonic: u8,
    /// Major, or natural minor.
    pub scale: PitchClasses,
    pub pentatonic: PitchClasses,
    /// The last chord guide chord at or before the tick, when its name converts to notes.
    pub chord: Option<Chord>,
}

impl Harmony {
    /// A missing chord offers the pentatonic instead.
    pub fn pitches(&self, set: Set) -> PitchClasses {
        match set {
            Set::Chromatic => PitchClasses::ALL,
            Set::Scale => self.scale,
            Set::Penta => self.pentatonic,
            Set::Chord => self.chord.as_ref().map_or(self.pentatonic, |c| c.pitches),
        }
    }
}

/// Chord name to pitch classes, remembered so a name converts once.
#[derive(Default)]
pub struct Chords(HashMap<String, Option<PitchClasses>>);

impl Chords {
    /// `None` when the name converts to no notes.
    pub fn pitches(&mut self, name: &str) -> Option<PitchClasses> {
        *self.0.entry(name.to_owned()).or_insert_with(|| {
            cmrt_chord::note_progression(name)
                .ok()
                .map(|chords| PitchClasses::of(chords.into_iter().flatten()))
                .filter(|p| !p.is_empty())
        })
    }
}

pub fn at(music: &Music, tick: u64) -> Harmony {
    at_with(
        music,
        &KeySignatures::new(music),
        &mut Chords::default(),
        tick,
    )
}

/// `at` with the song's key signatures and chord conversions already at hand.
pub fn at_with(music: &Music, keys: &KeySignatures, chords: &mut Chords, tick: u64) -> Harmony {
    let guide = chord_guide::track(music);
    let key = keys.map(guide.unwrap_or(0)).and_then(|m| m.at(tick));
    let Key { tonic, scale } = Key::from_signature(key);
    let name = chord_guide::chords(music)
        .filter(|e| e.tick <= tick)
        .max_by_key(|e| e.tick)
        .and_then(chord_guide::chord);
    Harmony {
        key,
        tonic: tonic % 12,
        scale: PitchClasses::on(tonic, scale.diatonic()),
        pentatonic: PitchClasses::on(tonic, Scale::steps(scale)),
        chord: name.and_then(|name| {
            Some(Chord {
                name: name.to_owned(),
                pitches: chords.pitches(name)?,
            })
        }),
    }
}

#[cfg(test)]
mod tests;
