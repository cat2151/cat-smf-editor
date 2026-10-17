use super::*;

fn drag(ctx: &egui::Context, app: &mut App, from: egui::Pos2, to: egui::Pos2) {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    render(ctx, app, vec![egui::Event::PointerMoved(from)]);
    render(ctx, app, vec![button(from, true)]);
    render(ctx, app, vec![egui::Event::PointerMoved(to)]);
    assert!(
        app.playback.repeat.is_none(),
        "range commits only on release"
    );
    render(ctx, app, vec![button(to, false)]);
}

#[test]
fn map_drag_selects_repeat_in_both_directions_and_preserves_cursor() {
    for (raw, split, reverse) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
    ] {
        let mut app = two_tracks();
        app.note_mode = !raw;
        if split {
            app.track_view.split = Some([0, 1]);
        }
        let ctx = egui::Context::default();
        render(&ctx, &mut app, vec![]);
        let output = render(&ctx, &mut app, vec![]);
        let frame = outlines(&output)[0];
        // Track 0 is short enough to show its full map outline.
        let a = egui::pos2(frame.center().x, frame.top() + frame.height() * 0.2);
        let b = egui::pos2(a.x, frame.top() + frame.height() * 0.7);
        let cursor = app.track_view.cursor_tick;
        drag(
            &ctx,
            &mut app,
            if reverse { b } else { a },
            if reverse { a } else { b },
        );
        let (a, b) = app.playback.repeat.expect("A-B repeat");
        assert!(a < b);
        assert!((a as i64 - 2784).abs() < 5, "{a}");
        assert!((b as i64 - 9744).abs() < 5, "{b}");
        assert_eq!(app.track_view.cursor_tick, cursor);
        let active = app
            .playback
            .pending
            .as_ref()
            .map(|p| &p.active)
            .or(app.playback.active.as_ref())
            .unwrap();
        assert!((active.start_seconds - a as f64 / 960.0).abs() < 0.01);
    }
}

#[test]
fn event_rows_drag_selects_repeat_without_scrolling_or_editing() {
    for raw in [false, true] {
        let mut app = two_tracks();
        app.note_mode = !raw;
        let ctx = egui::Context::default();
        render(&ctx, &mut app, vec![]);
        let output = render(&ctx, &mut app, vec![]);
        let positions: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t)
                    if t.galley.text().contains(':')
                        && t.galley
                            .text()
                            .trim_start()
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit()) =>
                {
                    Some(t.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .collect();
        assert!(positions.len() >= 4, "{positions:?}");
        let visible = app.track_view.visible_ticks;
        drag(&ctx, &mut app, positions[1], positions[3]);
        assert!(app.playback.repeat.is_some(), "raw={raw}");
        assert!(app.note_edit.is_none());
        assert_eq!(app.track_view.visible_ticks, visible);
    }
}
