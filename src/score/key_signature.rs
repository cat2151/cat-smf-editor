//! Key signatures at each tick, independent of note rendering and PPQN.
use crate::model::{Event, Music};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySignature {
    sharps: i8,
    minor: bool,
}

impl KeySignature {
    pub fn from_event(event: &Event) -> Option<Self> {
        if event.status != 0xff || event.meta_type != 0x59 {
            return None;
        }
        let [sharps, mode] = event.data.as_slice() else {
            return None;
        };
        let sharps = *sharps as i8;
        ((-7..=7).contains(&sharps) && *mode <= 1).then_some(Self {
            sharps,
            minor: *mode == 1,
        })
    }

    pub fn tonic(self) -> u8 {
        (7 * i16::from(self.sharps) + if self.minor { 9 } else { 0 }).rem_euclid(12) as u8
    }

    pub fn name(self) -> String {
        let names = if self.minor {
            [
                "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#",
            ]
        } else {
            [
                "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#",
            ]
        };
        format!(
            "{} {}",
            names[(self.sharps + 7) as usize],
            if self.minor { "minor" } else { "major" }
        )
    }

    pub fn minor(self) -> bool {
        self.minor
    }

    /// Fifths upward from the tonic on the twelve pitch-class circle (0..12).
    pub fn fifths_above_tonic(self, pitch: u8) -> u8 {
        (7 * (i16::from(pitch % 12) - i16::from(self.tonic()))).rem_euclid(12) as u8
    }
}

#[derive(Default)]
pub struct KeyMap {
    changes: Vec<(u64, KeySignature)>,
}

impl KeyMap {
    /// All changes at the queried tick have taken effect; last track/event wins.
    pub fn at(&self, tick: u64) -> Option<KeySignature> {
        self.changes
            .partition_point(|(t, _)| *t <= tick)
            .checked_sub(1)
            .map(|i| self.changes[i].1)
    }

    fn build(music: &Music, track: Option<usize>) -> Self {
        let mut changes = Vec::new();
        for (t, source) in music.tracks.iter().enumerate() {
            if track.is_some_and(|selected| selected != t) {
                continue;
            }
            for (i, event) in source.events.iter().enumerate() {
                if let Some(key) = KeySignature::from_event(event) {
                    changes.push((event.tick, t, i, key));
                }
            }
        }
        changes.sort_by_key(|&(tick, track, index, _)| (tick, track, index));
        Self {
            changes: changes
                .into_iter()
                .map(|(tick, _, _, key)| (tick, key))
                .collect(),
        }
    }
}

pub struct KeySignatures {
    maps: Vec<KeyMap>,
    independent: bool,
}

impl KeySignatures {
    pub fn new(music: &Music) -> Self {
        let independent = music.format == 2;
        let maps = if independent {
            (0..music.tracks.len())
                .map(|t| KeyMap::build(music, Some(t)))
                .collect()
        } else {
            vec![KeyMap::build(music, None)]
        };
        Self { maps, independent }
    }

    pub fn map(&self, track: usize) -> Option<&KeyMap> {
        self.maps.get(if self.independent { track } else { 0 })
    }
}
