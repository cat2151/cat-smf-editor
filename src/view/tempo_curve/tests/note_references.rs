use super::*;
use crate::view::piano_roll::REFERENCE_COLOR;

#[test]
fn first_track_with_notes_opens_piano_roll_with_gray_reference_tracks() {
    let mut music = music();
    music.tracks[0].events.extend(notes(&[(0, 480, 60)]).events);
    music.tracks[0].events.sort_by_key(|e| e.tick);
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
    let mut app = app(music);
    let original = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let (ctx, output) = opened_in(&mut app, 0);
    assert!(app.piano_roll.open);
    assert!(!app.tempo_curve.open);
    let gray: Vec<_> = output
        .shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match &s.shape {
            Shape::Rect(r) if r.fill == REFERENCE_COLOR => Some((i, r.rect)),
            _ => None,
        })
        .collect();
    assert_eq!(
        gray.len(),
        1,
        "only pitches within the first track's existing range"
    );
    let colored = output
        .shapes
        .iter()
        .enumerate()
        .find_map(|(i, s)| match &s.shape {
            Shape::Rect(r)
                if r.fill == crate::view::pitch_keyboard::roll_color(60, None)
                    && r.rect == gray[0].1 =>
            {
                Some(i)
            }
            _ => None,
        })
        .unwrap();
    assert!(gray.iter().all(|(i, _)| *i < colored));
    assert_eq!(
        cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap(),
        original
    );
    press(&mut app, &ctx, egui::Key::Escape);
    assert!(!app.piano_roll.open);
}

#[test]
fn conductor_detection_ignores_note_off_but_counts_unpaired_note_on() {
    for (status, velocity, conductor) in [(0x80, 80, true), (0x90, 0, true), (0x91, 80, false)] {
        let mut music = music();
        music.tracks[0]
            .events
            .insert(0, event(0, status, 0, &[60, velocity]));
        cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
        let mut app = app(music);
        opened_in(&mut app, 0);
        assert_eq!(app.tempo_curve.open, conductor);
        assert_eq!(app.piano_roll.open, !conductor);
    }
}

#[test]
fn a_note_free_track_after_the_first_opens_piano_roll() {
    let mut music = music();
    music.tracks[1] = Track {
        events: vec![event(3840, 0xff, 0x2f, &[])],
    };
    let mut app = app(music);
    opened_in(&mut app, 1);
    assert!(app.piano_roll.open);
    assert!(!app.tempo_curve.open);
}
