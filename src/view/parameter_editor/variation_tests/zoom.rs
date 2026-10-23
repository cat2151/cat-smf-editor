use super::*;

fn pointer(pos: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn left_bar_restores_reduced_variation_zoom_and_caps_click_wheel_and_drag_at_fit() {
    for gesture in 0..3 {
        let mut app = fixture();
        let ctx = egui::Context::default();
        for _ in 0..6 {
            frame(&mut app, &ctx, vec![]);
        }
        let initial = app.parameter_editor.viewport.axis.unwrap();
        let panel = app.parameter_editor.viewport.tempo_rect.unwrap();
        let pos = egui::pos2(panel.left() - 22.0, panel.center().y);
        let before = current(&app).unwrap();
        let generation = app.generation;
        let time_zoom = app.parameter_editor.viewport.time_zoom();
        // Two right clicks reduce the scale below 0.5x.
        for _ in 0..2 {
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
            frame(
                &mut app,
                &ctx,
                vec![pointer(pos, egui::PointerButton::Secondary, true)],
            );
            frame(
                &mut app,
                &ctx,
                vec![pointer(pos, egui::PointerButton::Secondary, false)],
            );
        }
        let reduced = app.parameter_editor.viewport.axis.unwrap();
        let initial_span = initial.high - initial.low;
        assert!((reduced.high - reduced.low) > initial_span * 2.0);
        for _ in 0..6 {
            match gesture {
                0 => {
                    frame(
                        &mut app,
                        &ctx,
                        vec![pointer(pos, egui::PointerButton::Primary, true)],
                    );
                    frame(
                        &mut app,
                        &ctx,
                        vec![pointer(pos, egui::PointerButton::Primary, false)],
                    );
                }
                1 => {
                    frame(
                        &mut app,
                        &ctx,
                        vec![egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, 100.0),
                            modifiers: egui::Modifiers::NONE,
                        }],
                    );
                }
                _ => {
                    let start = egui::pos2(pos.x, panel.top() + panel.height() * 0.2);
                    let end = egui::pos2(pos.x, panel.top() + panel.height() * 0.5);
                    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
                    frame(
                        &mut app,
                        &ctx,
                        vec![pointer(start, egui::PointerButton::Primary, true)],
                    );
                    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
                    frame(
                        &mut app,
                        &ctx,
                        vec![pointer(end, egui::PointerButton::Primary, false)],
                    );
                }
            }
        }
        let restored = app.parameter_editor.viewport.axis.unwrap();
        assert!(
            (restored.low - initial.low).abs() < 1e-9,
            "gesture={gesture}: {restored:?}"
        );
        assert!((restored.high - initial.high).abs() < 1e-9);
        assert_eq!(app.parameter_editor.viewport.time_zoom(), time_zoom);
        assert_eq!(current(&app).unwrap(), before);
        assert_eq!(app.generation, generation);
        assert!(!app.undo.available());
    }
}
