use super::*;

#[test]
fn maximized_curve_covers_the_event_map_to_the_bottom_edge() {
    let mut app = fixture();
    app.playback.repeat = Some((0, 240));
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let layer = egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("parameter-curve-maximized"),
    );
    for pos in [egui::pos2(650.0, 780.0), egui::pos2(650.0, 798.0)] {
        assert_eq!(ctx.layer_id_at(pos), Some(layer), "{pos:?}");
    }
    let start = egui::pos2(650.0, 780.0);
    let end = egui::pos2(650.0, 798.0);
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
    for (pos, pressed) in [(start, true), (end, false)] {
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(app.playback.repeat, Some((0, 240)));
    key(&mut app, &ctx, egui::Key::Escape);
    for _ in 0..20 {
        frame(&mut app, &ctx, vec![]);
    }
    assert!(!app.parameter_editor.open);
    assert_eq!(ctx.layer_id_at(end).unwrap().order, egui::Order::Background);
}
