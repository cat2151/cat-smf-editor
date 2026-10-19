use super::*;
use crate::view::piano_roll::{layout, panes, REFERENCE_COLOR};

fn multitrack() -> crate::app::App {
    let mut music = arpeggio().loaded.unwrap().music;
    let mut other = with_notes(&[
        (0, 240, 60),
        (720, 240, 61),
        (960, 480, 24),
        (1920, 240, 108),
    ])
    .loaded
    .unwrap()
    .music;
    music.tracks.push(other.tracks.remove(0));
    music
        .tracks
        .push(cat_smf_editor::model::Track { events: vec![] });
    app(music)
}

#[test]
fn references_stay_in_the_current_pitch_range_without_becoming_editable() {
    let app = multitrack();
    let loaded = app.loaded.as_ref().unwrap();
    let mut roll = layout(loaded, 0, 0);
    assert_eq!((roll.low, roll.high), (58, 74));
    assert_eq!(roll.notes.len(), NOTES.len());
    assert_eq!(roll.references.len(), 2);
    assert_eq!(roll.notes[0].1, roll.references[0]);
    assert!(hovered(&roll.notes, roll.references[1].center()).is_none());
    let empty = layout(loaded, 2, 0);
    assert!(empty.notes.is_empty() && empty.references.is_empty());
    let original = roll.references.clone();
    let height = roll.size.y;
    panes::fit_height(&mut roll, height * 2.0);
    for (before, after) in original.iter().zip(&roll.references) {
        assert_eq!(before.x_range(), after.x_range());
        assert_eq!(before.top() * 2.0, after.top());
        assert_eq!(before.bottom() * 2.0, after.bottom());
    }
    assert_eq!(roll.notes[0].1, roll.references[0]);
}

#[test]
fn references_are_gray_and_painted_behind_the_current_tracks_notes() {
    let mut app = multitrack();
    let (_, output) = opened_in(&mut app);
    let notes: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == REFERENCE_COLOR || r.stroke.width == 0.5 => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(notes.len(), 2 + NOTES.len());
    assert!(notes[..2].iter().all(|r| r.fill == REFERENCE_COLOR));
    assert!(notes[2..].iter().all(|r| r.fill != REFERENCE_COLOR));
    assert_eq!(notes[0].rect, notes[2].rect);
}
