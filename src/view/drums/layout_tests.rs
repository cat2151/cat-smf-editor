use super::*;

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
