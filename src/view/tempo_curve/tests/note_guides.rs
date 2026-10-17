use super::{music, opened_in};
use crate::view::{piano_roll::tests::at, row::beat_color, tests::app};
use eframe::egui::{self, epaint::Shape};

#[test]
fn guides_bracket_all_tracks_note_events_with_dim_dashes_and_gray_offs() {
    for zoomed in [false, true] {
        let mut app = app(music());
        let (ctx, _) = opened_in(&mut app, 0);
        if zoomed {
            app.tempo_curve.viewport.zoom_edge(
                0,
                true,
                super::super::Axis {
                    low: 120.0,
                    high: 150.0,
                },
                0.0,
            );
            app.tempo_curve.viewport.pending_scroll = Some(50.0);
            at(&mut app, &ctx, 11.0, vec![]);
        }
        let panel = app.tempo_curve.viewport.tempo_rect.unwrap();
        let scale = if zoomed { 0.3 } else { 0.2 };
        let origin = panel.left() - app.tempo_curve.viewport.scroll;
        for (tick, expected) in [
            (700.0, vec![(480.0, Some(2)), (960.0, Some(3))]),
            (1500.0, vec![(960.0, Some(3)), (1920.0, None)]),
        ] {
            let output = at(
                &mut app,
                &ctx,
                12.0,
                vec![egui::Event::PointerMoved(egui::pos2(
                    origin + tick * scale,
                    panel.center().y,
                ))],
            );
            let lines: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::LineSegment { points, stroke }
                        if stroke.width == 1.5
                            && points[0].x == points[1].x
                            && stroke.color != beat_color(1) =>
                    {
                        assert!(s.clip_rect.top() >= panel.top() - 0.01);
                        assert!(s.clip_rect.bottom() <= panel.bottom() + 0.01);
                        Some((points, stroke))
                    }
                    _ => None,
                })
                .collect();
            for (tick, on) in expected {
                let x = origin + tick * scale;
                let segments: Vec<_> = lines
                    .iter()
                    .filter(|(p, _)| (p[0].x - x).abs() < 0.01)
                    .collect();
                if let Some(beat) = on {
                    let base = beat_color(beat);
                    let dim = |v: u8| (f32::from(v) * 0.55) as u8;
                    let color =
                        egui::Color32::from_rgb(dim(base.r()), dim(base.g()), dim(base.b()));
                    assert!(segments.len() > 2);
                    assert!(segments
                        .iter()
                        .all(|(p, s)| s.color == color && (p[1].y - p[0].y).abs() <= 5.01));
                } else {
                    assert_eq!(segments.len(), 1);
                    assert_eq!(segments[0].1.color, super::super::note_guides::OFF_COLOR);
                    assert!((segments[0].0[1].y - segments[0].0[0].y).abs() > 50.0);
                }
            }
        }
        let output = at(
            &mut app,
            &ctx,
            13.0,
            vec![egui::Event::PointerMoved(egui::Pos2::ZERO)],
        );
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape,
            Shape::LineSegment { points, stroke } if stroke.width == 1.5 && points[0].x == points[1].x && stroke.color != beat_color(1))));
    }
}
