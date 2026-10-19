use super::*;
use crate::view::piano_roll::REFERENCE_COLOR;

#[test]
fn a_conductor_with_notes_has_colored_notes_over_gray_reference_tracks() {
    let mut music = music();
    music.tracks[0].events.extend(notes(&[(0, 480, 60)]).events);
    music.tracks[0].events.sort_by_key(|e| e.tick);
    cat_smf_editor::edit::refresh_deltas(&mut music.tracks[0]).unwrap();
    let mut app = app(music);
    let original = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
    let (_, output) = opened_in(&mut app, 0);
    assert!(app.tempo_curve.open);
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
        "only pitches within the conductor's existing range"
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
}
