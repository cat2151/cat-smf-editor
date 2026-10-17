//! Dragging a note's parts on the roll.
use super::{arpeggio, at, beat_lines, near, note_rects, opened_in, NOTES};
use crate::view::piano_roll::hover::Part;
use eframe::egui;

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// Presses at `from`, moves to `to` and releases, one frame per step from `time`.
/// Returns a frame drawn while the button is still down at `to`.
pub(super) fn drag(
    app: &mut crate::app::App,
    ctx: &egui::Context,
    time: f64,
    from: egui::Pos2,
    to: egui::Pos2,
) -> egui::FullOutput {
    let steps = [
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![egui::Event::PointerMoved(from + (to - from) * 0.5)],
        vec![egui::Event::PointerMoved(to)],
    ];
    for (i, events) in steps.into_iter().enumerate() {
        at(app, ctx, time + i as f64 * 0.01, events);
    }
    let held = at(app, ctx, time + 0.04, vec![]);
    at(app, ctx, time + 0.05, vec![button(to, false)]);
    at(app, ctx, time + 0.06, vec![]);
    held
}

/// (tick, duration, key) of every note on the roll's track, in event order.
pub(super) fn notes_of(app: &crate::app::App) -> Vec<(u64, u64, u8)> {
    let loaded = app.loaded.as_ref().unwrap();
    let events = &loaded.music.tracks[0].events;
    loaded.note_views[0]
        .durations
        .iter()
        .enumerate()
        .filter_map(|(i, d)| d.map(|d| (events[i].tick, d, events[i].data[0])))
        .collect()
}

fn ticks_of(app: &crate::app::App) -> Vec<u64> {
    app.loaded.as_ref().unwrap().music.tracks[0]
        .events
        .iter()
        .map(|e| e.tick)
        .collect()
}

#[test]
fn dragging_each_part_changes_the_start_the_position_or_the_duration_on_release() {
    // G4 at 480 for 480 ticks; 24px is 120 ticks at 96px per 480 ticks.
    for (part, grab, dx, expected) in [
        (Part::Start, 24.0, 24.0, (600, 360)),
        (Part::Start, 24.0, -24.0, (360, 600)),
        (Part::Middle, 48.0, 24.0, (600, 480)),
        (Part::Middle, 48.0, -24.0, (360, 480)),
        (Part::End, 72.0, 24.0, (480, 600)),
        (Part::End, 72.0, -24.0, (480, 360)),
    ] {
        let mut app = arpeggio();
        let (ctx, output) = opened_in(&mut app);
        let note = note_rects(&output)[2];
        let from = egui::pos2(note.left() + grab, note.center().y);
        let held = drag(&mut app, &ctx, 11.0, from, from + egui::vec2(dx, 0.0));
        // While held, the dragged part's rectangle follows the pointer and the track is untouched.
        let drawn = drag_rect(&held, note.y_range());
        let shifted = |x: f32, moves: bool| if moves { x + dx } else { x };
        let want = egui::Rect::from_x_y_ranges(
            shifted(note.left(), part != Part::End)..=shifted(note.right(), part != Part::Start),
            note.y_range(),
        );
        assert!(
            near(drawn, want),
            "{part:?} {dx}: drew {drawn:?}, want {want:?}"
        );
        let mut expected_notes = NOTES.to_vec();
        expected_notes[2] = (expected.0, expected.1, 67);
        expected_notes.sort_by_key(|n| n.0);
        let mut got = notes_of(&app);
        got.sort_by_key(|n| n.0);
        assert_eq!(got, expected_notes, "{part:?} {dx}");
        assert!(app.piano_roll.error.is_none());
    }
}

/// The note-colored rectangle on the row spanning `rows`.
fn drag_rect(output: &egui::FullOutput, rows: egui::Rangef) -> egui::Rect {
    let found: Vec<_> = note_rects(output)
        .into_iter()
        .filter(|r| r.y_range() == rows)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    found[0]
}

#[test]
fn a_drag_across_another_note_of_the_same_key_leaves_the_track_and_shows_an_error() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let before = ticks_of(&app);
    // C4 at 0..240 (48px) dragged by its middle onto the other C4 at 3840..4800.
    let rects = note_rects(&output);
    let note = rects[0];
    let from = egui::pos2(note.center().x, note.center().y);
    drag(&mut app, &ctx, 11.0, from, from + egui::vec2(800.0, 0.0));
    assert_eq!(ticks_of(&app), before);
    assert!(app.piano_roll.error.is_some());
    // The error line above the roll pushes every note down by the same amount;
    // the dragged note is back where it was relative to the others.
    let after = note_rects(&at(&mut app, &ctx, 12.0, vec![]));
    let down = after[1].top() - rects[1].top();
    for (a, b) in after.iter().zip(&rects) {
        assert!(
            near(*a, b.translate(egui::vec2(0.0, down))),
            "{b:?} drawn at {a:?}"
        );
    }
    // The next drag clears the error.
    let from = egui::pos2(after[0].center().x, after[0].center().y);
    drag(&mut app, &ctx, 13.0, from, from + egui::vec2(4.0, 0.0));
    assert!(app.piano_roll.error.is_none());
    assert_eq!(notes_of(&app)[0], (20, 240, 60));
}

#[test]
fn dragging_empty_space_or_back_to_the_grab_changes_nothing() {
    let mut app = arpeggio();
    let (ctx, output) = opened_in(&mut app);
    let before = ticks_of(&app);
    let generation = app.generation;
    let note = note_rects(&output)[2];
    let empty = egui::pos2(note.right() + 30.0, note.center().y);
    // No range is selected when the empty-space drag returns to its origin.
    drag(&mut app, &ctx, 11.0, empty, empty);
    let middle = note.center();
    drag(&mut app, &ctx, 12.0, middle, middle);
    assert_eq!(ticks_of(&app), before);
    assert_eq!(app.generation, generation);
}

#[test]
fn an_applied_drag_keeps_the_roll_scrolled_where_it_was() {
    let mut app = arpeggio();
    crate::cursor::split::go(&mut app, 0, 3840);
    let (ctx, output) = opened_in(&mut app);
    let measure_3 = beat_lines(&output)[8].0;
    // The C4 in measure 3, 960 ticks = 192px, lengthened from its end by 24px.
    let note = note_rects(&output)[4];
    let from = egui::pos2(note.right() - 4.0, note.center().y);
    drag(&mut app, &ctx, 11.0, from, from + egui::vec2(24.0, 0.0));
    assert_eq!(notes_of(&app)[4], (3840, 1080, 60));
    let after = at(&mut app, &ctx, 12.0, vec![]);
    assert!((beat_lines(&after)[8].0 - measure_3).abs() < 0.01);
}
