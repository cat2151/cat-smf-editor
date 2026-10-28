use super::*;
use crate::view::patterns::edit_tests::fixture;
use cat_smf_editor::score::{note_view::note_on, pattern};

#[test]
fn pattern_edit_piano_drag_maps_destination_position_and_length_to_source() {
    for (part, dx, expected) in [(0, 24.0, (240, 480)), (1, 24.0, (120, 600))] {
        let (mut a, source, id) = fixture();
        crate::view::patterns::library::edit(&mut a, id).unwrap();
        let (c, output) = opened_in(&mut a);
        let notes = note_rects(&output);
        assert_eq!(notes.len(), 1);
        let note = notes[0];
        let from = if part == 0 {
            note.center()
        } else {
            egui::pos2(note.right() - note.width() / 6.0, note.center().y)
        };
        drag::drag(&mut a, &c, 11.0, from, from + egui::vec2(dx, 0.0));
        assert!(a.piano_roll.error.is_none(), "{:?}", a.piano_roll.error);
        let l = a.loaded.as_ref().unwrap();
        let on = l.music.tracks[source]
            .events
            .iter()
            .position(|e| note_on(e) && e.tick < 1920)
            .unwrap();
        let durations =
            cat_smf_editor::score::note_view::NoteView::new(&l.music.tracks[source].events);
        assert_eq!(
            (
                l.music.tracks[source].events[on].tick,
                durations.durations[on].unwrap()
            ),
            expected
        );
        assert!(!l.music.tracks[0]
            .events
            .iter()
            .any(|e| note_on(e) && e.tick >= 1920));
    }
}

#[test]
fn pattern_edit_piano_drag_rejects_boundary_and_drops_old_gesture_after_take_switch() {
    let (mut a, _, id) = fixture();
    crate::view::patterns::library::edit(&mut a, id).unwrap();
    let (c, output) = opened_in(&mut a);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let note = note_rects(&output)[0];
    let from = egui::pos2(note.right() - note.width() / 6.0, note.center().y);
    drag::drag(&mut a, &c, 11.0, from, from + egui::vec2(400.0, 0.0));
    assert!(a.piano_roll.error.is_some());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    let output = at(&mut a, &c, 12.0, vec![]);
    let from = note_rects(&output)[0].center();
    at(
        &mut a,
        &c,
        13.0,
        vec![
            egui::Event::PointerMoved(from),
            egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let other = pattern::Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .keys()
        .copied()
        .find(|&p| p != id)
        .unwrap();
    crate::view::patterns::library::edit(&mut a, other).unwrap();
    let switched = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    at(
        &mut a,
        &c,
        13.1,
        vec![
            egui::Event::PointerMoved(from + egui::vec2(24.0, 0.0)),
            egui::Event::PointerButton {
                pos: from + egui::vec2(24.0, 0.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        switched
    );
}
