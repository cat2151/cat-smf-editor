use super::*;
use crate::{edit, model::Event};

fn key(tick: u64, sharps: i8, minor: bool) -> Event {
    Event {
        tick,
        delta: 0,
        status: 0xff,
        meta_type: 0x59,
        data: vec![sharps as u8, u8::from(minor)],
    }
}

fn classes(names: &[u8]) -> PitchClasses {
    PitchClasses::of(names.iter().copied())
}

/// A new song (480 ticks per quarter) with `keys` in the conductor and, when given, a chord guide.
fn song(keys: &[Event], chords: Option<&[(u64, &str)]>) -> Music {
    let mut music = edit::tracks::new_song();
    for k in keys {
        edit::insert(&mut music.tracks[0], k.clone());
    }
    if let Some(chords) = chords {
        chord_guide::add(&mut music).unwrap();
        let guide = chord_guide::track(&music).unwrap();
        for &(tick, text) in chords {
            edit::insert(
                &mut music.tracks[guide],
                chord_guide::event(tick, text).unwrap(),
            );
        }
    }
    music
}

#[test]
fn key_signatures_give_the_key_scale_and_pentatonic_where_they_take_effect() {
    // G major from the start, E minor from measure 2 (tick 1920).
    let music = song(&[key(0, 1, false), key(1920, 1, true)], None);
    let g = at(&music, 1919);
    assert_eq!(g.key.map(|k| (k.tonic(), k.minor())), Some((7, false)));
    assert_eq!(g.tonic, 7);
    assert_eq!(g.scale, classes(&[7, 9, 11, 0, 2, 4, 6]));
    assert_eq!(g.pentatonic, classes(&[7, 9, 11, 2, 4]));
    let e = at(&music, 1920);
    assert_eq!(e.key.map(|k| (k.tonic(), k.minor())), Some((4, true)));
    assert_eq!(e.tonic, 4);
    // E natural minor: E F# G A B C D; E minor pentatonic: E G A B D.
    assert_eq!(e.scale, classes(&[4, 6, 7, 9, 11, 0, 2]));
    assert_eq!(e.pentatonic, classes(&[4, 7, 9, 11, 2]));
    // F major (one flat) and D minor.
    let music = song(&[key(0, -1, false), key(960, -1, true)], None);
    assert_eq!(at(&music, 0).scale, classes(&[5, 7, 9, 10, 0, 2, 4]));
    assert_eq!(at(&music, 0).pentatonic, classes(&[5, 7, 9, 0, 2]));
    assert_eq!(at(&music, 960).scale, classes(&[2, 4, 5, 7, 9, 10, 0]));
    assert_eq!(at(&music, 960).pentatonic, classes(&[2, 5, 7, 9, 0]));
}

#[test]
fn a_chord_on_every_beat_gives_each_beat_its_own_chord() {
    let music = song(
        &[key(0, 0, false)],
        Some(&[(0, "C"), (480, "Am"), (960, "F"), (1440, "G7")]),
    );
    let expected: [(&str, &[u8]); 4] = [
        ("C", &[0, 4, 7]),
        ("Am", &[9, 0, 4]),
        ("F", &[5, 9, 0]),
        ("G7", &[7, 11, 2, 5]),
    ];
    for (beat, (name, pitches)) in expected.into_iter().enumerate() {
        for tick in [beat as u64 * 480, beat as u64 * 480 + 479] {
            let h = at(&music, tick);
            let chord = h.chord.clone().unwrap();
            assert_eq!(
                (chord.name.as_str(), chord.pitches),
                (name, classes(pitches))
            );
            assert_eq!(h.pitches(Set::Chord), classes(pitches));
        }
    }
    // The last chord lasts until the next one.
    assert_eq!(at(&music, 100_000).chord.unwrap().name, "G7");
}

#[test]
fn without_a_chord_the_chord_set_is_the_pentatonic() {
    // No chord guide at all.
    let music = song(&[key(0, 2, false)], None);
    let h = at(&music, 0);
    assert_eq!(h.chord, None);
    assert_eq!(h.pitches(Set::Chord), classes(&[2, 4, 6, 9, 11]));
    // A guide whose first chord comes later.
    let music = song(&[key(0, 0, false)], Some(&[(960, "F")]));
    assert_eq!(
        at(&music, 959).pitches(Set::Chord),
        classes(&[0, 2, 4, 7, 9])
    );
    assert_eq!(at(&music, 960).pitches(Set::Chord), classes(&[5, 9, 0]));
}

#[test]
fn without_a_key_signature_the_key_is_c_major() {
    let music = song(&[], None);
    let h = at(&music, 0);
    assert_eq!(h.key, None);
    assert_eq!(h.tonic, 0);
    assert_eq!(h.pitches(Set::Scale), classes(&[0, 2, 4, 5, 7, 9, 11]));
    assert_eq!(h.pitches(Set::Penta), classes(&[0, 2, 4, 7, 9]));
    assert_eq!(h.pitches(Set::Chord), classes(&[0, 2, 4, 7, 9]));
    assert_eq!(h.pitches(Set::Chromatic), PitchClasses::ALL);
}

#[test]
fn sets_cycle_from_chromatic_to_chord_and_back() {
    let mut set = Set::Chromatic;
    let mut seen = vec![set];
    for _ in 0..4 {
        set = set.next();
        seen.push(set);
    }
    use Set::*;
    assert_eq!(seen, [Chromatic, Scale, Penta, Chord, Chromatic]);
    assert_eq!(Set::default(), Chord);
}
