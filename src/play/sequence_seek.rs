//! Rebuild channel controls and pedal-held voices before resuming a MIDI stream.
use super::audio_sequence::MidiEvent;
use std::collections::BTreeMap;

struct Voice {
    word: u32,
    down: bool,
    sostenuto: bool,
}
fn channel(word: u32) -> usize {
    ((word >> 16) & 15) as usize
}
fn key(word: u32) -> u32 {
    word & 0x000f_7f00
}
fn on(word: u32) -> bool {
    (word >> 20) & 15 == 9 && word & 127 != 0
}
fn off(word: u32) -> bool {
    (word >> 20) & 15 == 8 || ((word >> 20) & 15 == 9 && word & 127 == 0)
}
pub(super) fn prelude(events: &[MidiEvent], origin: f64) -> Vec<MidiEvent> {
    let mut controls = BTreeMap::new();
    let mut voices: Vec<Voice> = vec![];
    let mut sustain = [false; 16];
    let mut sostenuto = [false; 16];
    for e in events.iter().take_while(|e| e.time < origin) {
        let ch = channel(e.word);
        let kind = (e.word >> 20) & 15;
        if on(e.word) {
            voices.push(Voice {
                word: e.word,
                down: true,
                sostenuto: false,
            });
        } else if off(e.word) {
            if let Some(v) = voices
                .iter_mut()
                .find(|v| v.down && key(v.word) == key(e.word))
            {
                v.down = false;
            }
        } else if kind == 11 || kind == 14 {
            let cc = if kind == 11 { (e.word >> 8) & 127 } else { 128 };
            controls.insert((ch, cc), e.word);
            if cc == 64 {
                sustain[ch] = e.word & 127 >= 64;
            }
            if cc == 66 {
                let active = e.word & 127 >= 64;
                if active != sostenuto[ch] {
                    for v in voices.iter_mut().filter(|v| channel(v.word) == ch) {
                        v.sostenuto = active && v.down;
                    }
                }
                sostenuto[ch] = active;
            }
        }
        voices.retain(|v| v.down || sustain[channel(v.word)] || v.sostenuto);
    }
    // An ordinary release exactly at the seek position must not restart a voice.
    for e in events.iter().filter(|e| e.time == origin && off(e.word)) {
        if let Some(i) = voices
            .iter()
            .position(|v| v.down && key(v.word) == key(e.word))
            && !sustain[channel(e.word)]
            && !voices[i].sostenuto
        {
            voices.remove(i);
        }
    }
    let mut result = vec![];
    let mut push = |word| result.push(MidiEvent { time: origin, word });
    for (&(_, cc), &word) in &controls {
        if cc != 66 {
            push(word);
        }
    }
    // Sostenuto captures only the voices already down when the pedal rises.
    for v in voices.iter().filter(|v| v.sostenuto) {
        push(v.word);
    }
    for (&(_, cc), &word) in &controls {
        if cc == 66 {
            push(word);
        }
    }
    for v in voices.iter().filter(|v| !v.sostenuto) {
        push(v.word);
    }
    for v in voices.iter().filter(|v| !v.down) {
        push(0x2080_0000 | key(v.word));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(words: &[(f64, u32)], seek: f64) -> Vec<u32> {
        prelude(
            &words
                .iter()
                .map(|&(time, word)| MidiEvent { time, word })
                .collect::<Vec<_>>(),
            seek,
        )
        .into_iter()
        .map(|e| {
            assert_eq!(e.time, seek);
            e.word
        })
        .collect()
    }
    #[test]
    fn sustain_restores_released_notes_after_channel_control_snapshot() {
        let events = [
            (0., 0x20b1_407f),
            (0.1, 0x2091_3c64),
            (0.2, 0x2081_3c26),
            (0.3, 0x20e1_0060),
            (0.4, 0x20b1_0750),
        ];
        assert_eq!(
            run(&events, 0.5),
            vec![
                0x20b1_0750,
                0x20b1_407f,
                0x20e1_0060,
                0x2091_3c64,
                0x2081_3c00
            ]
        );
        let mut released = events.to_vec();
        released.push((0.45, 0x20b1_4000));
        assert_eq!(
            run(&released, 0.5),
            vec![0x20b1_0750, 0x20b1_4000, 0x20e1_0060]
        );
    }
    #[test]
    fn sostenuto_restores_only_captured_voices_and_same_pitch_multiplicity() {
        let events = [
            (0., 0x2090_3c64),
            (0.1, 0x20b0_427f),
            (0.2, 0x2090_3c50),
            (0.3, 0x2080_3c20),
            (0.4, 0x2090_4064),
        ];
        assert_eq!(
            run(&events, 0.5),
            vec![
                0x2090_3c64,
                0x20b0_427f,
                0x2090_3c50,
                0x2090_4064,
                0x2080_3c00
            ]
        );
    }
}
