use super::*;

#[test]
fn cursor_changes_color_only_while_drums_own_focus() {
    let mut app = super::tests::fixture();
    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    // Keep a green note under the cursor throughout focus changes.
    activate(&mut app).unwrap();
    let mut frame = |time, focused, browser| {
        app.player_selection.open = browser;
        let mut input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .focused = Some(focused);
        let mut color = None;
        for pass in 0..3 {
            let output = ctx.run(input.clone(), |ctx| widgets::show(&mut app, ctx, &mut None));
            let (_, _, cells) = ctx
                .data(|d| {
                    d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                        "drum-grid-bounds",
                    ))
                })
                .unwrap();
            let selected = cells[3 * 18 + 2];
            let active = focused && !browser;
            if pass == 2 {
                assert_eq!(
                    output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::epaint::Shape::Rect(rect)
                        if rect.rect == selected.shrink(3.0)
                            && rect.stroke == egui::Stroke::new(1.0_f32, egui::Color32::from_gray(24))
                    )),
                    active,
                );
            }
            color = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(rect)
                    if rect.rect == selected
                        && rect.stroke.width == if active { 3.0 } else { 2.0 } =>
                {
                    assert_eq!(rect.fill, egui::Color32::from_rgb(166, 226, 46));
                    Some(rect.stroke.color)
                }
                _ => None,
            });
        }
        color.expect("selected cell must have a cursor frame")
    };
    let orange = crate::view::ACTIVE_PANE;
    assert_eq!(frame(0.0, true, false), orange);
    assert_eq!(frame(0.35, true, false), egui::Color32::WHITE);
    assert_eq!(frame(0.7, true, false), orange);
    assert_eq!(frame(1.0, false, false), orange);
    assert_eq!(frame(1.5, false, false), orange);
    assert_eq!(frame(2.0, true, true), orange);
    assert_eq!(frame(2.5, true, true), orange);
    assert_eq!(frame(3.85, true, false), egui::Color32::WHITE);
}

#[test]
fn all_cells_fit_after_viewport_resize_without_scrolling() {
    let mut app = super::tests::fixture();
    let ctx = egui::Context::default();
    for width in [1280.0, 800.0, 640.0, 1100.0] {
        for _ in 0..3 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 600.0),
                    )),
                    ..Default::default()
                },
                |c| show(&mut app, c),
            );
        }
        let (area, clip, cells) = ctx
            .data(|d| {
                d.get_temp::<(egui::Rect, egui::Rect, Vec<egui::Rect>)>(egui::Id::new(
                    "drum-grid-bounds",
                ))
            })
            .unwrap();
        assert_eq!(cells.len(), 4 * 18);
        assert!(
            area.left() >= 0.0 && area.right() <= width,
            "Grid outside viewport: {area:?}"
        );
        assert!(
            cells
                .iter()
                .all(|cell| area.contains_rect(*cell) && clip.contains_rect(*cell)),
            "Clipped cells at width {width}"
        );
        assert!(cells
            .iter()
            .all(|cell| cell.width() >= 10.0 && cell.height() >= 30.0));
        app.drum_editor.field = 17;
        super::commands::apply(&mut app, super::commands::Op::Toggle);
        assert!(playback::measure(&app).unwrap().active(
            &app.loaded.as_ref().unwrap().music.tracks[app.track],
            0,
            15
        ));
        super::commands::apply(&mut app, super::commands::Op::Toggle);
    }
}
