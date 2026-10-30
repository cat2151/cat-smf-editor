use super::*;

#[test]
fn a_disabled_row_rejects_coalesced_pointer_input_and_releases_its_gesture() {
    let ctx = egui::Context::default();
    let mut drag = State::default();
    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(300.0, 24.0));
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: Default::default(),
    };
    let from = rect.left_center() + egui::vec2(10.0, 0.0);
    let to = rect.center();
    for events in [
        vec![],
        vec![
            button(from, true),
            egui::Event::PointerMoved(to),
            button(to, false),
        ],
    ] {
        let _ = ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.disable();
                    assert!(row(ui, rect, &1.0_f32, None, &mut drag).is_none());
                });
            },
        );
    }
    assert!(drag.is_none());
}
