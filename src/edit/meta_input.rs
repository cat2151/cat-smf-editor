//! Human-readable input for standard SMF meta events.
use crate::model::Event;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Tempo,
    Chord,
    KeySignature,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chord => "Chord",
            Self::Tempo => "Tempo",
            Self::KeySignature => "Key signature",
        }
    }
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Chord => "Isus4",
            Self::Tempo => "120",
            Self::KeySignature => "D minor",
        }
    }
    pub fn event(self, tick: u64, input: &str) -> Result<Event, String> {
        if self == Self::Chord {
            return crate::score::chord_guide::event(tick, input);
        }
        let (meta_type, data) = match self {
            Self::Chord => unreachable!(),
            Self::Tempo => {
                let bpm = input.trim().parse::<f64>().map_err(|_| "Invalid BPM")?;
                let micros = (60_000_000.0 / bpm).round();
                if !bpm.is_finite() || bpm <= 0.0 || !(1.0..=16_777_215.0).contains(&micros) {
                    return Err("BPM is outside the MIDI tempo range".into());
                }
                (0x51, (micros as u32).to_be_bytes()[1..].to_vec())
            }
            Self::KeySignature => {
                let text = input.trim().replace('♯', "#").replace('♭', "b");
                let words: Vec<_> = text.split_whitespace().collect();
                let [root, mode] = words.as_slice() else {
                    return Err("Invalid key signature (e.g. D minor)".into());
                };
                let minor = if mode.eq_ignore_ascii_case("minor") {
                    true
                } else if mode.eq_ignore_ascii_case("major") {
                    false
                } else {
                    return Err("Expected major or minor".into());
                };
                let names = if minor {
                    [
                        "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B", "F#", "C#", "G#",
                        "D#", "A#",
                    ]
                } else {
                    [
                        "Cb", "Gb", "Db", "Ab", "Eb", "Bb", "F", "C", "G", "D", "A", "E", "B",
                        "F#", "C#",
                    ]
                };
                let index = names
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(root))
                    .ok_or("Key signature must use at most seven sharps or flats")?;
                (0x59, vec![(index as i8 - 7) as u8, u8::from(minor)])
            }
        };
        Ok(Event {
            tick,
            delta: 0,
            status: 0xff,
            meta_type,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tempo_rounds_to_24_bit_microseconds() {
        assert_eq!(Kind::Tempo.event(42, "120").unwrap().data, [7, 161, 32]);
        let e = Kind::Tempo.event(42, "123.45").unwrap();
        assert_eq!(e.tick, 42);
        assert_eq!(e.data, (486027u32).to_be_bytes()[1..]);
        for bad in ["NaN", "inf", "0", "-1", "1", "999999999", ""] {
            assert!(Kind::Tempo.event(0, bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn keys_encode_signed_accidentals_and_mode() {
        for (text, expected) in [
            ("D minor", [255, 1]),
            ("C major", [0, 0]),
            ("C♭ major", [249, 0]),
            ("A# minor", [7, 1]),
            (" f♯ MAJOR ", [6, 0]),
        ] {
            let e = Kind::KeySignature.event(0, text).unwrap();
            assert_eq!(e.meta_type, 0x59);
            assert_eq!(e.data, expected);
        }
        for bad in ["D", "D dorian", "G# major", "D minor extra"] {
            assert!(Kind::KeySignature.event(0, bad).is_err());
        }
    }
}
