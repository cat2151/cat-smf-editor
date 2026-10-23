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
fn outside_controls_roundtrip_and_drive_steeper_native_bounded_curves() {
    let mut app = fixture();
    for value in [-4.0, 5.0] {
        let mut c = index_curve();
        c.points.truncate(2);
        c.controls.truncate(1);
        c.move_control(0, 240, value).unwrap();
        let track = &mut app.loaded.as_mut().unwrap().music.tracks[0];
        let before = macro_values::read(track).unwrap();
        macro_curve::write(track, &c).unwrap();
        let bytes = cat_smf_editor::smf_io::encode(&app.loaded.as_ref().unwrap().music).unwrap();
        let restored = cat_smf_editor::ffi::parse(&bytes).unwrap();
        assert_eq!(macro_curve::read(&restored.tracks[0]).unwrap(), vec![c]);
        assert_eq!(macro_values::read(&restored.tracks[0]).unwrap(), before);
        let lane = &macro_curve::expand(&restored.tracks[0]).unwrap()[0];
        // Extrapolate the control geometry rather than clipping the control.
        let expected = (0.2_f64 + 0.6 * (value * 0.5 + 0.25)).clamp(0.0, 1.0);
        assert!((lane.value_at(240.0).unwrap() - expected).abs() < 1e-10);
        for tick in 0..=480 {
            assert!((0.0..=1.0).contains(&lane.value_at(f64::from(tick)).unwrap()));
        }
    }
}

#[test]
fn keyboard_control_crosses_both_limits_and_supports_cancel_confirm_undo() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let mut c = index_curve();
    c.controls[0].value = c.minimum;
    c.controls[1].value = c.maximum;
    macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &c).unwrap();
    for (index, direction, expected) in [
        (0, egui::Key::ArrowDown, -0.02),
        (1, egui::Key::ArrowUp, 2.02),
    ] {
        app.parameter_editor.selected = Some((crate::view::tempo_curve::Target::Control, index));
        key(&mut app, &ctx, egui::Key::Enter);
        key(&mut app, &ctx, direction);
        key(&mut app, &ctx, egui::Key::Escape);
        assert_eq!(current(&app).unwrap(), c);
        key(&mut app, &ctx, egui::Key::Enter);
        key(&mut app, &ctx, direction);
        key(&mut app, &ctx, egui::Key::Enter);
        assert!((current(&app).unwrap().controls[index].value - expected).abs() < 1e-10);
        app.undo_music().unwrap();
        assert_eq!(current(&app).unwrap(), c);
    }
    key(&mut app, &ctx, egui::Key::R);
    assert!(!app.parameter_editor.open);
}

#[test]
fn dragging_controls_above_and_below_variation_range_preserves_values() {
    for value in [-0.2, 2.2] {
        let mut app = fixture();
        let ctx = egui::Context::default();
        let mut c = index_curve();
        c.controls[0].value = 0.5;
        macro_curve::write(&mut app.loaded.as_mut().unwrap().music.tracks[0], &c).unwrap();
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let output = frame(&mut app, &ctx, vec![]);
        let from = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::epaint::Shape::QuadraticBezier(b) => Some(b.points[1]),
                _ => None,
            })
            .unwrap();
        let rect = app.parameter_editor.viewport.tempo_rect.unwrap();
        let axis = app.parameter_editor.viewport.axis.unwrap();
        let to = egui::pos2(
            from.x,
            rect.bottom() - ((value - axis.low) / (axis.high - axis.low)) as f32 * rect.height(),
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(from)]);
        frame(
            &mut app,
            &ctx,
            vec![pointer(from, egui::PointerButton::Primary, true)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(to)]);
        frame(
            &mut app,
            &ctx,
            vec![pointer(to, egui::PointerButton::Primary, false)],
        );
        let after = current(&app).unwrap();
        assert!((after.controls[0].value - value).abs() < 1e-5, "{after:?}");
        assert_eq!(after.points, c.points);
        app.undo_music().unwrap();
        assert_eq!(current(&app).unwrap(), c);
    }
}
