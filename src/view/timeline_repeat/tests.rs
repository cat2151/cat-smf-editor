use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut State,
    range: Option<(u64, u64)>,
    time: f64,
    events: Vec<egui::Event>,
) -> Option<Action> {
    let mut action = None;
    let _ = ctx.run(
        egui::RawInput {
            time: Some(time),
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 160.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                action = row(
                    ui,
                    egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1000.0, 24.0)),
                    &1.0_f32,
                    range,
                    state,
                )
                .or(action);
            });
        },
    );
    action
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn thirty_hz_right_drag_keeps_range_feedback_but_restarts_at_four_hz_and_flushes_release() {
    let ctx = egui::Context::default();
    let mut state = State::default();
    let mut range = Some((100, 800));
    let origin = egui::pos2(800.0, 32.0);
    frame(
        &ctx,
        &mut state,
        range,
        0.0,
        vec![egui::Event::PointerMoved(origin)],
    );
    frame(&ctx, &mut state, range, 0.01, vec![button(origin, true)]);
    let mut restarts = Vec::new();
    let mut previews = 0;
    for i in 1..=30 {
        let time = 0.02 + i as f64 / 30.0;
        let pos = origin - egui::vec2(20.0 + i as f32 * 3.0, 0.0);
        match frame(
            &ctx,
            &mut state,
            range,
            time,
            vec![egui::Event::PointerMoved(pos)],
        ) {
            Some(Action::Start(r)) => {
                range = Some(r);
                restarts.push(time);
            }
            Some(Action::Preview(r)) => {
                range = Some(r);
                previews += 1;
            }
            _ => {}
        }
        assert_eq!(
            range,
            Some((100, (pos.x - 10.0) as u64)),
            "range must follow every move"
        );
    }
    assert_eq!(
        restarts.len(),
        4,
        "forced restarts during one second of dragging"
    );
    assert!(restarts.windows(2).all(|t| t[1] - t[0] >= 0.25));
    assert!(previews >= 20);
    let final_pos = origin - egui::vec2(115.0, 0.0);
    let action = frame(
        &ctx,
        &mut state,
        range,
        1.04,
        vec![
            egui::Event::PointerMoved(final_pos),
            button(final_pos, false),
        ],
    );
    assert!(matches!(action, Some(Action::Start((100, 675)))));
    assert!(state.is_none());
}

#[test]
fn pending_drag_change_is_applied_when_pointer_stops_and_duplicate_release_does_not_restart() {
    let ctx = egui::Context::default();
    let mut state = State::default();
    let mut range = Some((100, 800));
    let origin = egui::pos2(800.0, 32.0);
    frame(
        &ctx,
        &mut state,
        range,
        0.0,
        vec![egui::Event::PointerMoved(origin)],
    );
    frame(&ctx, &mut state, range, 0.01, vec![button(origin, true)]);
    let first = egui::pos2(780.0, 32.0);
    assert!(matches!(
        frame(
            &ctx,
            &mut state,
            range,
            0.02,
            vec![egui::Event::PointerMoved(first)]
        ),
        Some(Action::Start((100, 770)))
    ));
    range = Some((100, 770));
    let next = egui::pos2(750.0, 32.0);
    assert!(matches!(
        frame(
            &ctx,
            &mut state,
            range,
            0.04,
            vec![egui::Event::PointerMoved(next)]
        ),
        Some(Action::Preview((100, 740)))
    ));
    range = Some((100, 740));
    assert!(matches!(
        frame(&ctx, &mut state, range, 0.28, vec![]),
        Some(Action::Start((100, 740)))
    ));
    assert!(frame(&ctx, &mut state, range, 0.29, vec![button(next, false)]).is_none());
    assert!(state.is_none());
}
