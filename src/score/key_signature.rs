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

    /// Sharps when positive, flats when negative.
    pub fn sharps(self) -> i8 {
        self.sharps
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
        Self::excluding(music, track, None)
    }

    fn excluding(music: &Music, track: Option<usize>, excluded: Option<usize>) -> Self {
        let mut changes = Vec::new();
        for (t, source) in music.tracks.iter().enumerate() {
            if excluded == Some(t) || track.is_some_and(|selected| selected != t) {
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
        let independent = music.format == 2 || crate::score::chord_guide::supported(music);
        if crate::score::chord_guide::supported(music) {
            let own: Vec<_> = (0..music.tracks.len())
                .map(|t| KeyMap::build(music, Some(t)))
                .collect();
            let guide = crate::score::chord_guide::track(music);
            let maps = (0..own.len())
                .map(|t| {
                    if crate::score::pattern::is_source(music, t) {
                        return KeyMap::excluding(music, None, guide);
                    }
                    let mut ticks: Vec<_> = own
                        .iter()
                        .flat_map(|m| m.changes.iter().map(|c| c.0))
                        .collect();
                    ticks.sort_unstable();
                    ticks.dedup();
                    let changes = ticks
                        .into_iter()
                        .filter_map(|tick| {
                            let key = own[t]
                                .at(tick)
                                .or_else(|| {
                                    (t != 0)
                                        .then(|| {
                                            guide.filter(|&g| g != t).and_then(|g| own[g].at(tick))
                                        })
                                        .flatten()
                                })
                                .or_else(|| (t != 0).then(|| own[0].at(tick)).flatten());
                            key.map(|k| (tick, k))
                        })
                        .collect();
                    KeyMap { changes }
                })
                .collect();
            return Self { maps, independent };
        }
        let maps = if independent {
            (0..music.tracks.len())
                .map(|t| KeyMap::build(music, Some(t)))
                .collect()
        } else {
            vec![KeyMap::build(music, None)]
        };
        Self { maps, independent }
    }

    /// Pattern/Scene local time retains its existing key interpretation, without guide propagation.
    pub fn local_timeline(music: &Music) -> Self {
        let independent = music.format == 2;
        let maps = if independent {
            (0..music.tracks.len())
                .map(|t| KeyMap::build(music, Some(t)))
                .collect()
        } else {
            vec![KeyMap::excluding(
                music,
                None,
                crate::score::chord_guide::track(music),
            )]
        };
        Self { maps, independent }
    }

    pub fn map(&self, track: usize) -> Option<&KeyMap> {
        self.maps.get(if self.independent { track } else { 0 })
    }
}
