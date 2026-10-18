use super::*;

#[test]
fn exact_chords_use_one_row_with_a_trailing_length_and_spaces() {
    let tracks = vec![vec![note(0, 960, 79), note(0, 960, 60), note(0, 960, 64)]];
    let output = rows(&tracks, Some((0, 960)), 480, &[0, 480]);
    assert_eq!(output.len(), 1);
    assert_eq!(text(&output[0].tokens), " 'ce<g2' ");
    assert!(output[0].tokens.iter().all(|t| t.tick == 0));
    for playing in [None, Some(0), Some(240), Some(960)] {
        let painted = job(output[0].tokens.iter(), playing, 480);
        for note in &tracks[0] {
            let expected = Palette::dark_note(note.color, score::lit(0, 960, playing, 480));
            assert!(painted.sections.iter().any(|s| s.format.color == expected));
        }
    }
}

#[test]
fn chord_durations_are_explicit_even_among_default_length_notes() {
    let tracks = vec![vec![
        note(0, 480, 60),
        note(480, 960, 62),
        note(960, 1440, 64),
        note(1440, 1920, 60),
        note(1440, 1920, 64),
        note(1440, 1920, 67),
        note(1920, 2640, 60),
        note(1920, 2640, 64),
        note(1920, 2640, 67),
    ]];
    let output = rows(&tracks, Some((0, 2640)), 480, &[]);
    assert_eq!(text(&output[0].tokens), "l4 cde 'ceg4'  'ceg4.' ");
}

#[test]
fn close_timings_and_clipped_timings_do_not_become_chords() {
    for notes in [
        vec![note(0, 960, 60), note(0, 959, 64)],
        vec![note(0, 960, 60), note(1, 961, 64)],
        vec![note(0, 960, 60), note(0, 1440, 64)],
        vec![note(0, 960, 60), note(240, 960, 64)],
    ] {
        let output = rows(&[notes], Some((480, 960)), 480, &[]);
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|r| !text(&r.tokens).contains('\'')));
    }
    let output = rows(
        &[vec![note(0, 960, 60)], vec![note(0, 960, 64)]],
        Some((0, 960)),
        480,
        &[],
    );
    assert_eq!(output.len(), 2);
}

#[test]
fn octave_steps_are_local_to_each_chord_and_support_both_directions() {
    let notes = [
        note(0, 960, 84),
        note(0, 960, 62),
        note(0, 960, 67),
        note(960, 1440, 24),
    ];
    assert_eq!(
        text(&cell(&notes.iter().collect::<Vec<_>>(), 0, 1440, 480)),
        " 'c>>dg2' c4"
    );
    let tracks = vec![vec![
        note(0, 960, 60),
        note(0, 960, 84),
        note(960, 1920, 60),
        note(960, 1920, 64),
        note(960, 1920, 67),
    ]];
    assert_eq!(
        text(&rows(&tracks, Some((0, 1920)), 480, &[])[0].tokens),
        "c2 'ceg2' "
    );
}

#[test]
fn octave_doublings_collapse_to_single_notes_or_distinct_chord_names() {
    for (pitches, expected) in [
        (vec![60, 72, 84], vec!["c4"]),
        (vec![60, 64, 72, 76], vec!["e4", "c4"]),
        (vec![60, 72, 79], vec!["g4", "c4"]),
        (vec![60, 61, 72, 73], vec!["c+4", "c4"]),
        (vec![60, 64, 67, 72, 76, 79], vec![" 'ceg4' "]),
    ] {
        let notes = pitches.into_iter().map(|p| note(0, 480, p)).collect();
        let output = rows(&[notes], Some((0, 480)), 480, &[]);
        assert_eq!(
            output.iter().map(|r| text(&r.tokens)).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn octave_doubled_melodies_use_normal_default_lengths() {
    let notes = [60, 62, 64]
        .into_iter()
        .enumerate()
        .flat_map(|(i, pitch)| {
            [pitch, pitch + 12].map(|p| note(i as u64 * 480, (i as u64 + 1) * 480, p))
        })
        .collect();
    let output = rows(&[notes], Some((0, 1440)), 480, &[]);
    assert_eq!(output.len(), 1);
    assert_eq!(text(&output[0].tokens), "l4 cde");
}

#[test]
fn only_three_or_more_names_at_least_an_eighth_long_form_chords() {
    for (ppqn, duration, count, expected_rows) in [
        (480, 239, 3, 3),
        (480, 240, 3, 1),
        (480, 120, 4, 4),
        (480, 960, 2, 2),
        (481, 240, 3, 3),
        (481, 241, 3, 1),
    ] {
        let notes = [60, 64, 67, 71]
            .into_iter()
            .take(count)
            .map(|p| note(0, duration, p))
            .collect();
        let output = rows(&[notes], Some((0, duration)), ppqn, &[]);
        assert_eq!(
            output.len(),
            expected_rows,
            "ppqn={ppqn}, duration={duration}, count={count}"
        );
        assert_eq!(
            output.iter().any(|r| text(&r.tokens).contains('\'')),
            expected_rows == 1
        );
    }
}

#[test]
fn dyad_melodies_remain_two_high_first_lines() {
    let notes = [60, 62, 64]
        .into_iter()
        .enumerate()
        .flat_map(|(i, pitch)| {
            [pitch, pitch + 7].map(|p| note(i as u64 * 480, (i as u64 + 1) * 480, p))
        })
        .collect();
    let output = rows(&[notes], Some((0, 1440)), 480, &[]);
    assert_eq!(
        output.iter().map(|r| text(&r.tokens)).collect::<Vec<_>>(),
        ["l4 gab", "l4 cde"]
    );
}

#[test]
fn a_long_chord_remains_a_chord_in_a_short_measure_fragment() {
    let notes = [60, 64, 67].into_iter().map(|p| note(0, 960, p)).collect();
    let output = rows(&[notes], Some((900, 1920)), 480, &[]);
    assert_eq!(output.len(), 1);
    assert!(text(&output[0].tokens).starts_with(" 'ceg32' "));
}

#[test]
fn shortened_quarters_place_rests_in_the_following_beats() {
    let tracks = vec![vec![note(0, 336, 62), note(768, 1104, 62)]];
    let output = rows(&tracks, Some((0, 1536)), 384, &[0, 384, 768, 1152]);
    let cells: Vec<_> = (0..4)
        .map(|beat| {
            job(
                output[0]
                    .tokens
                    .iter()
                    .filter(|t| t.tick >= beat * 384 && t.tick < (beat + 1) * 384),
                None,
                384,
            )
            .text
        })
        .collect();
    assert_eq!(cells, ["l4 d", "r", "d", "r"]);
}
