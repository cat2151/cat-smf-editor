use super::navigation::{click, wheel};
use super::*;

#[test]
fn only_drags_starting_in_empty_space_zoom_even_when_crossing_notes_in_either_direction() {
    for reverse in [false, true] {
        let mut app = arpeggio();
        let (ctx, output) = opened_in(&mut app);
        let panel = app.piano_roll.viewport.panel.unwrap();
        let note = note_rects(&output)[2];
        let mut a = note.center() - egui::vec2(note.width() / 2.0 + 30.0, 0.0);
        let mut b = note.center() + egui::vec2(note.width() / 2.0 + 30.0, 0.0);
        if reverse {
            std::mem::swap(&mut a, &mut b);
        }
        let before = super::drag::notes_of(&app);
        let generation = app.generation;
        super::drag::drag(&mut app, &ctx, 11.0, a, b);
        let output = at(&mut app, &ctx, 12.0, vec![]);
        let next = note_rects(&output)[2];
        assert!(
            (next.width() / note.width() - panel.width() * 0.9 / (b.x - a.x).abs()).abs() < 0.01
        );
        assert_eq!(super::drag::notes_of(&app), before);
        assert_eq!(app.generation, generation);
        assert!(!app.playback.follow);
        assert!(app.playback.active.is_none() && app.playback.pending.is_none());
        let output = wheel(&mut app, &ctx, 13.0, panel.center(), -20.0);
        assert!((note_rects(&output)[2].width() - next.width()).abs() < 0.01);
    }
}

#[test]
fn right_click_and_drag_adjust_each_view_end_without_editing_or_auditioning() {
    for (right, dragging) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut app = arpeggio();
        let (ctx, _) = opened_in(&mut app);
        app.piano_roll.viewport.pending = Some(egui::vec2(500.0, 0.0));
        let output = at(&mut app, &ctx, 11.0, vec![]);
        let panel = app.piano_roll.viewport.panel.unwrap();
        let note = note_rects(&output)[2];
        let old_origin = note.left() - 480.0 * 0.2;
        let fraction = if right { 0.8 } else { 0.2 };
        let pos = egui::pos2(panel.left() + panel.width() * fraction, panel.top() + 4.0);
        let before = app.generation;
        if dragging {
            let from = pos + egui::vec2(if right { -20.0 } else { 20.0 }, 0.0);
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Secondary,
                modifiers: egui::Modifiers::NONE,
            };
            at(&mut app, &ctx, 12.0, vec![egui::Event::PointerMoved(from)]);
            at(&mut app, &ctx, 12.1, vec![button(from, true)]);
            let held = at(&mut app, &ctx, 12.2, vec![egui::Event::PointerMoved(pos)]);
            assert_eq!(note_rects(&held)[2].left(), note.left());
            at(&mut app, &ctx, 12.3, vec![button(pos, false)]);
        } else {
            click(&mut app, &ctx, 12.0, pos, true);
        }
        let output = at(&mut app, &ctx, 13.0, vec![]);
        let next = note_rects(&output)[2];
        let scale = next.width() / 480.0;
        let tick = (pos.x - old_origin) / 0.2;
        let (left, end) = if right {
            (500.0 / 0.2, tick)
        } else {
            (tick, (500.0 + panel.width()) / 0.2)
        };
        assert!(((end - left) * scale - panel.width() * 0.9).abs() < 0.2);
        assert!(
            (next.left() - 480.0 * scale + left * scale - (panel.left() + panel.width() * 0.05))
                .abs()
                < 0.2
        );
        assert_eq!(app.generation, before);
        assert!(app.playback.active.is_none() && app.playback.pending.is_none());
    }
}
