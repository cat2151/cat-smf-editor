use super::*;
use crate::view::tempo_curve::Axis;

fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| crate::view::show(app, ctx),
    )
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn native_axis_bars_share_tempo_navigation_without_editing() {
    let mut app = tests::app();
    let ctx = egui::Context::default();
    // Use a native plain range that cannot be mistaken for BPM or normalized values.
    let mut c = current(&app).unwrap();
    c.minimum = -100.0;
    c.maximum = -20.0;
    for p in c.points.iter_mut().chain(&mut c.controls) {
        p.value = -100.0 + p.value * 80.0;
    }
    store(&mut app, c.clone());
    for _ in 0..6 {
        frame(&mut app, &ctx, vec![]);
    }
    let output = frame(&mut app, &ctx, vec![]);
    let bars: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.fill == egui::Color32::from_gray(75) => Some(r.rect),
            _ => None,
        })
        .collect();
    let panel = app.parameter_editor.viewport.tempo_rect.unwrap();
    let vertical = *bars
        .iter()
        .find(|r| r.height() > r.width() && (panel.left() - r.right() - 10.0).abs() < 1.0)
        .expect("parameter value zoom bar");
    let horizontal = *bars
        .iter()
        .find(|r| r.width() > r.height() && (r.left() - vertical.left()).abs() < 1.0)
        .expect("parameter time zoom bar");
    let generation = app.generation;
    let time_zoom = app.parameter_editor.viewport.time_zoom();
    let pos = vertical.center();
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
    frame(&mut app, &ctx, vec![button(pos, true)]);
    frame(&mut app, &ctx, vec![button(pos, false)]);
    let view = &app.parameter_editor.viewport;
    let axis = view
        .axis
        .unwrap_or_else(|| panic!("bars={bars:?}, panel={:?}", view.tempo_rect));
    assert!(axis.low >= -100.0 && axis.high <= -20.0);
    assert!((axis.high - axis.low - 80.0 / 1.5).abs() < 1e-6);
    assert_eq!(view.time_zoom(), time_zoom);
    let pos = horizontal.center();
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
    frame(&mut app, &ctx, vec![button(pos, true)]);
    frame(&mut app, &ctx, vec![button(pos, false)]);
    assert!(app.parameter_editor.viewport.time_zoom() > time_zoom);
    assert_eq!(app.parameter_editor.viewport.axis, Some(axis));
    frame(&mut app, &ctx, vec![]);
    assert!(app
        .parameter_editor
        .labels
        .iter()
        .all(|(v, _)| *v >= axis.low && *v <= axis.high));
    assert_eq!(current(&app).unwrap(), c);
    assert_eq!(app.generation, generation);
}

#[test]
fn content_wheel_scrolls_time_after_value_zoom_and_notes_drag_fits_time() {
    let mut app = tests::app();
    let ctx = egui::Context::default();
    let c = current(&app).unwrap();
    let generation = app.generation;
    app.parameter_editor.viewport.axis = Some(Axis {
        low: 0.1,
        high: 0.9,
    });
    // Ensure the song is wider than the viewport, without altering curve data.
    let loaded = app.loaded.as_mut().unwrap();
    loaded.music.tracks[0].events.last_mut().unwrap().tick = 192_000;
    cat_smf_editor::edit::refresh_deltas(&mut loaded.music.tracks[0]).unwrap();
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let panel = app.parameter_editor.viewport.tempo_rect.unwrap();
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(panel.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -100.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..20 {
        frame(&mut app, &ctx, vec![]);
    }
    assert!(app.parameter_editor.viewport.scroll > 0.0);
    assert_eq!(app.parameter_editor.viewport.time_zoom(), 1.0);
    assert_eq!(
        app.parameter_editor.viewport.axis,
        Some(Axis {
            low: 0.1,
            high: 0.9
        })
    );
    let panel = app.parameter_editor.viewport.tempo_rect.unwrap();
    let start = egui::pos2(panel.left() + 100.0, panel.top() - 30.0);
    let end = start + egui::vec2(200.0, 0.0);
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
    frame(&mut app, &ctx, vec![button(start, true)]);
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
    frame(&mut app, &ctx, vec![button(end, false)]);
    assert!(app.parameter_editor.viewport.time_zoom() > 1.0);
    assert_eq!(current(&app).unwrap(), c);
    assert_eq!(app.generation, generation);
    assert!(app.playback.pending.is_none());
}
