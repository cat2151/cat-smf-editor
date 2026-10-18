use super::*;
use crate::view::staff::notation;

pub(super) fn note(tick: u64, end: u64, pitch: u8) -> Note {
    Note {
        tick,
        end,
        spelled: notation::spell(pitch, 0),
        value: 0,
        accidental: None,
        color: crate::view::pitch_keyboard::roll_color(pitch, None),
    }
}

fn text(tokens: &Tokens) -> String {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

#[test]
fn requested_example_is_l24_c4de() {
    let notes = [note(0, 480, 60), note(480, 560, 62), note(560, 640, 64)];
    assert_eq!(
        text(&cell(&notes.iter().collect::<Vec<_>>(), 0, 640, 480)),
        "l24 c4de"
    );
}

#[test]
fn simple_rows_omit_unhelpful_l_and_tiny_rests() {
    assert_eq!(text(&cell(&[], 0, 1920, 480)), "");
    let d = note(0, 960, 62);
    assert_eq!(text(&cell(&[&d], 0, 960, 480)), "d2");
    let notes = [note(0, 480, 60), note(495, 975, 62)];
    assert_eq!(
        text(&cell(&notes.iter().collect::<Vec<_>>(), 0, 990, 480)),
        "c4d4"
    );
    let notes = [note(0, 480, 60), note(960, 1440, 62)];
    assert_eq!(
        text(&cell(&notes.iter().collect::<Vec<_>>(), 0, 1440, 480)),
        "l4 crd"
    );
}

#[test]
fn approximate_lengths_group_timing_variations_and_use_only_sibling_notation() {
    let notes = [note(0, 73, 60), note(80, 162, 62), note(162, 640, 64)];
    let tokens = cell(&notes.iter().collect::<Vec<_>>(), 0, 640, 480);
    assert!(text(&tokens).starts_with("l24 "));
    for pitch in 0..=127 {
        let notes = [note(0, 73, pitch), note(73, 793, 127 - pitch)];
        let tokens = cell(&notes.iter().collect::<Vec<_>>(), 0, 1920, 480);
        assert!(text(&tokens)
            .chars()
            .all(|c| "lcdefgabr+-. 0123456789".contains(c)));
    }
    assert_eq!(
        length(720, 480),
        Length {
            denominator: 4,
            dotted: true
        }
    );
    assert_eq!(
        length(73, 480),
        Length {
            denominator: 24,
            dotted: false
        }
    );
}

#[test]
fn octaves_are_omitted_even_for_wide_leaps() {
    let notes = [note(0, 480, 24), note(480, 960, 72), note(960, 1440, 60)];
    assert_eq!(
        text(&cell(&notes.iter().collect::<Vec<_>>(), 0, 1440, 480)),
        "l4 ccc"
    );
}

#[test]
fn voices_are_rebuilt_per_measure_instead_of_inheriting_old_chords() {
    let notes = vec![
        note(480, 960, 62),
        note(0, 960, 67),
        note(0, 480, 60),
        note(0, 480, 64),
        note(480, 1440, 65),
        note(1920, 2400, 69),
    ];
    let voices = split(notes.iter(), 480);
    assert_eq!(voices.len(), 3);
    assert_eq!(voices.iter().map(Vec::len).sum::<usize>(), notes.len());
    assert!(voices.iter().all(|v| v
        .windows(2)
        .all(|w| w[0].end <= w[1].tick || (w[0].tick == w[1].tick && w[0].end == w[1].end))));
    let tracks = vec![notes, vec![note(0, 1920, 48)]];
    let first = rows(&tracks, Some((0, 1920)), 480, &[]);
    let next = rows(&tracks, Some((1920, 3840)), 480, &[]);
    assert_eq!(
        first.iter().map(|r| r.track).collect::<Vec<_>>(),
        [0, 0, 0, 1]
    );
    assert_eq!(first.len(), 4);
    assert_eq!(next.len(), 2);
    assert!(next[1].tokens.is_empty());
    assert!(!text(&first[0].tokens).contains('a'));
    assert!(next.iter().any(|row| text(&row.tokens).contains('a')));
}

#[test]
fn active_rows_are_highest_first_across_tracks_and_silent_rows_are_last() {
    let tracks = vec![
        vec![note(1920, 3840, 84)],
        vec![note(0, 1920, 60)],
        vec![note(0, 1920, 67), note(0, 1920, 72)],
    ];
    let rows = rows(&tracks, Some((0, 1920)), 480, &[]);
    assert_eq!(
        rows.iter().map(|r| r.track).collect::<Vec<_>>(),
        [2, 2, 1, 0]
    );
    assert_eq!(
        rows.iter().map(|r| text(&r.tokens)).collect::<Vec<_>>(),
        ["c1", "g1", "c1", ""]
    );
}

#[test]
fn a_monophonic_passage_with_a_carried_note_uses_one_row() {
    let tracks = vec![vec![
        note(0, 480, 48),
        note(0, 480, 55),
        note(0, 480, 60),
        note(1680, 2160, 72),
        note(2160, 2640, 74),
        note(2640, 3120, 76),
    ]];
    let rows = rows(&tracks, Some((1920, 3840)), 480, &[]);
    assert_eq!(rows.len(), 1);
    let sounding: String = rows[0]
        .tokens
        .iter()
        .filter(|t| t.tone.is_some())
        .map(|t| t.text.chars().next().unwrap())
        .collect();
    assert_eq!(sounding, "cde");
}

#[test]
fn measure_crossings_clip_without_foreign_tie_syntax() {
    let tracks = vec![vec![note(1440, 2400, 60), note(2400, 2880, 62)]];
    assert_eq!(
        text(&rows(&tracks, Some((0, 1920)), 480, &[])[0].tokens),
        "r2.c4"
    );
    assert_eq!(
        text(&rows(&tracks, Some((1920, 3840)), 480, &[])[0].tokens),
        "c4d4r2"
    );
}

#[test]
fn colors_match_the_dark_staff_for_idle_playing_and_ended_notes() {
    let tracks = vec![vec![], vec![note(0, 480, 60)], vec![note(0, 480, 64)]];
    let rows = rows(&tracks, Some((0, 1920)), 480, &[]);
    for row in &rows {
        let tone = tracks[row.track][0].color;
        for (tick, expected) in [
            (None, Palette::tinted(tone)),
            (Some(0), tone),
            (Some(240), tone),
            (Some(479), tone),
            (Some(480), Palette::tinted(tone)),
        ] {
            let painted = job(row.tokens.iter(), tick);
            assert!(painted.sections.iter().any(|s| s.format.color == expected));
        }
    }
}

#[test]
fn a_late_note_cannot_follow_a_rounded_up_whole_rest_in_four_four() {
    let note = note(1800, 1920, 60);
    assert_eq!(text(&cell(&[&note], 0, 1920, 480)), "r2.c16");
    assert!(cell(&[], 0, 1920, 480).is_empty());
}

#[test]
fn each_attack_is_assigned_to_its_actual_beat() {
    let tracks = vec![vec![
        note(0, 480, 60),
        note(960, 1440, 62),
        note(1800, 1920, 64),
    ]];
    let rows = rows(&tracks, Some((0, 1920)), 480, &[0, 480, 960, 1440]);
    let row = &rows[0];
    let cells: Vec<_> = (0..4)
        .map(|beat| {
            job(
                row.tokens
                    .iter()
                    .filter(|t| t.tick >= beat * 480 && t.tick < (beat + 1) * 480),
                None,
            )
            .text
        })
        .collect();
    assert_eq!(cells, ["l4 c", "r", "d", "r8.e16"]);
}

#[path = "ui_tests.rs"]
mod ui;

#[path = "chord_tests.rs"]
mod chords;
