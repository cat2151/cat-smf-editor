//! What a recording session plays and how long a take lasts; kept in the app status, never in SMF.
use super::{Modulation, Velocity};
use crate::score::key_signature::KeySignature;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Range {
    Free,
    /// Each lap of `measures` 4/4 bars becomes its own take.
    Loop {
        measures: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Start {
    /// Pads only sound until the Rec button.
    Button,
    /// The first pad also starts recording.
    Key,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// Four keyboard rows `z`, `a`, `q`, `1`.
    Rows,
    /// `c d e f g a b`.
    Letters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Note,
    /// Every row plays root position, each row one octave above the previous.
    Triad,
    Seventh,
    /// Rows play root position, then the first, second and third inversion.
    TriadInversions,
    SeventhInversions,
}

impl Mode {
    pub const ALL: [Self; 5] = [
        Self::Note,
        Self::Triad,
        Self::Seventh,
        Self::TriadInversions,
        Self::SeventhInversions,
    ];
    /// Notes each chord pad stacks; `None` plays single notes.
    pub fn chord_notes(self) -> Option<usize> {
        match self {
            Self::Note => None,
            Self::Triad | Self::TriadInversions => Some(3),
            Self::Seventh | Self::SeventhInversions => Some(4),
        }
    }
    pub fn inversions(self) -> bool {
        matches!(self, Self::TriadInversions | Self::SeventhInversions)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Triad => "Triad",
            Self::Seventh => "Seventh",
            Self::TriadInversions => "Triad inversions",
            Self::SeventhInversions => "Seventh inversions",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scale {
    MajorPentatonic,
    MinorPentatonic,
}

impl Scale {
    /// Semitones above the tonic of each pad degree.
    pub fn steps(self) -> &'static [u8] {
        match self {
            Self::MajorPentatonic => &[0, 2, 4, 7, 9],
            Self::MinorPentatonic => &[0, 3, 5, 7, 10],
        }
    }
    /// Parent scale whose degrees chord mode stacks in thirds.
    pub fn diatonic(self) -> &'static [u8] {
        match self {
            Self::MajorPentatonic => &[0, 2, 4, 5, 7, 9, 11],
            Self::MinorPentatonic => &[0, 2, 3, 5, 7, 8, 10],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    /// Pitch class 0..12.
    pub tonic: u8,
    pub scale: Scale,
}

impl Key {
    /// C major when the song declares no key.
    pub fn from_signature(signature: Option<KeySignature>) -> Self {
        signature.map_or(
            Self {
                tonic: 0,
                scale: Scale::MajorPentatonic,
            },
            |k| Self {
                tonic: k.tonic(),
                scale: if k.minor() {
                    Scale::MinorPentatonic
                } else {
                    Scale::MajorPentatonic
                },
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub range: Range,
    pub start: Start,
    /// Microseconds per quarter note, as in an SMF tempo event.
    pub tempo: u32,
    pub layout: Layout,
    pub mode: Mode,
    key: Key,
    /// Pitch of `z` in note mode, and the octave of the letter layout.
    pub note_base: u8,
    /// Root of the `z` chord in chord mode.
    pub chord_base: u8,
    /// Pads play the drum lanes; follows the container, never chosen.
    pub drums: bool,
    pub velocity: Velocity,
    pub modulation: Modulation,
    /// Takes and phrase loops start and end their notes on sixteenths.
    #[serde(default)]
    pub quantize: bool,
}

/// What the pads of one container play, kept per container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voicing {
    pub mode: Mode,
    pub note_base: u8,
    pub chord_base: u8,
}

impl Settings {
    pub fn voicing(&self) -> Voicing {
        Voicing {
            mode: self.mode,
            note_base: self.note_base,
            chord_base: self.chord_base,
        }
    }
    pub fn set_voicing(&mut self, v: Voicing) {
        (self.mode, self.note_base, self.chord_base) = (v.mode, v.note_base, v.chord_base);
    }
    /// Bases start on the tonic of octave 4 (notes) and octave 3 (chords).
    pub fn new(key: Key, tempo: u32) -> Self {
        let tonic = key.tonic % 12;
        Self {
            range: Range::Free,
            start: Start::Button,
            tempo,
            layout: Layout::Rows,
            mode: Mode::Note,
            key: Key { tonic, ..key },
            drums: false,
            note_base: 60 + tonic,
            chord_base: 48 + tonic,
            velocity: Velocity::Range80To127,
            modulation: Modulation::Sweep,
            quantize: false,
        }
    }
    pub fn key(&self) -> Key {
        self.key
    }
    /// Both bases move to the new tonic within their own octave.
    pub fn set_key(&mut self, key: Key) {
        let tonic = key.tonic % 12;
        self.key = Key { tonic, ..key };
        self.note_base = on_tonic(self.note_base, tonic);
        self.chord_base = on_tonic(self.chord_base, tonic);
    }
}

fn on_tonic(base: u8, tonic: u8) -> u8 {
    let pitch = base / 12 * 12 + tonic;
    if pitch > 127 {
        pitch - 12
    } else {
        pitch
    }
}
