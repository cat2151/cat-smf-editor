use crate::{cursor::split::tests::press, view::tests::app};
use cat_smf_editor::model::{Event, Music, Track};
use eframe::egui;

#[test]
fn split_panes_center_cursor_and_playhead_on_one_line() {
    let notes = |step: u64, count: u64| Track {
        events: (0..=count)
            .map(|i| Event {
                tick: i * step,
                delta: 0,
                status: if i < count { 0x90 } else { 0xff },
                meta_type: if i < count { 0 } else { 0x2f },
                data: if i < count { vec![60, 100] } else { vec![] },
            })
            .collect(),
    };
    let mut app = app(Music {
        format: 1,
        division: 480,
        tracks: vec![notes(60, 400), notes(480, 60)],
    });
    let ctx = egui::Context::default();
    let none = egui::Modifiers::NONE;
    press(&mut app, &ctx, &[]);
    press(
        &mut app,
        &ctx,
        &[(egui::Key::W, none), (egui::Key::H, none)],
    );
    for _ in 0..193 {
        press(&mut app, &ctx, &[(egui::Key::J, none)]);
    }
    let render = |app: &mut crate::app::App| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1900.0, 700.0),
                )),
                ..Default::default()
            },
            |ctx| crate::view::show(app, ctx),
        )
    };
    let rows_at = |output: &egui::FullOutput, mbt: &str| -> Vec<f32> {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) if t.galley.text().replace(' ', "") == mbt => {
                    Some(t.pos.y)
                }
                _ => None,
            })
            .collect()
    };
    // 7:1:0 is row 192 on the left and row 24 on the right.
    let output = render(&mut app);
    let outlines: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::Rect(r) if r.stroke.color == crate::view::ACTIVE_PANE => {
                Some(r.rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(outlines.len(), 1);
    // The active outline occupies the left half of the available track area.
    assert!(
        outlines[0].right() < (outlines[0].left() + 1900.0) / 2.0,
        "the left pane is active: {outlines:?}"
    );
    let y = rows_at(&output, "7:1:0");
    assert_eq!(y.len(), 2);
    assert_eq!(y[0], y[1], "left and right cursors are on different lines");
    assert!((y[0] - 350.0).abs() < 60.0, "cursor is not centered: {y:?}");
    // 2 s at 120 BPM later both playheads (9:1:0) share the same center line.
    crate::play::playback::start(&mut app, crate::play::playback::Mode::Position);
    render(&mut app);
    app.audition.set_test_seconds(2.0);
    let output = render(&mut app);
    let heads: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::epaint::Shape::LineSegment { points, stroke }
                if stroke.width == 4.0 && stroke.color == egui::Color32::LIGHT_GREEN =>
            {
                Some(points[0].y)
            }
            _ => None,
        })
        .collect();
    assert_eq!(heads.len(), 2);
    assert_eq!(heads[0], heads[1], "playheads are on different lines");
    assert!((heads[0] - y[0]).abs() < 20.0);
}
