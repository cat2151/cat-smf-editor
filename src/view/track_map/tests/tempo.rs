use super::*;
use cat_smf_editor::edit::tempo_curve::{self, Curve, Point};

#[test]
fn curve_replaces_only_conductor_tempo_rows_in_notes_and_raw_maps() {
    let curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 960,
                bpm: 60.0,
            },
            Point {
                tick: 10000,
                bpm: 120.0,
            },
        ],
        controls: vec![Point {
            tick: 4000,
            bpm: 180.0,
        }],
    };
    let mut music = two_tracks().loaded.unwrap().music;
    tempo_curve::write(&mut music, Some(&curve)).unwrap();
    tempo_curve::regenerate(&mut music, &curve).unwrap();
    let tempo_count = music.tracks[0]
        .events
        .iter()
        .filter(|e| e.meta_type == 0x51)
        .count();
    assert!(tempo_count > 10);
    for note_mode in [true, false] {
        for with_curve in [true, false] {
            let mut music = music.clone();
            if !with_curve {
                tempo_curve::write(&mut music, None).unwrap();
            }
            let mut app = app(music);
            app.note_mode = note_mode;
            let ctx = egui::Context::default();
            let mut rect = egui::Rect::NOTHING;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(120.0, 600.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        rect = ui.available_rect_before_wrap();
                        crate::view::track_map::show(&app, ui, 0);
                    });
                },
            );
            // The marker and track 1's independent tempo remain, above the curve.
            let rows = filled(&output, DARK)
                .into_iter()
                .filter(|r| r.width() == rect.width())
                .count();
            assert_eq!(rows, 2 + if with_curve { 0 } else { tempo_count });
            let curves: Vec<_> = output
                .shapes
                .iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    if let egui::Shape::QuadraticBezier(b) = &s.shape {
                        Some((i, b))
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(curves.len(), usize::from(with_curve));
            if with_curve {
                let (index, shape) = curves[0];
                assert_eq!(
                    shape.stroke.color,
                    egui::epaint::ColorMode::Solid(crate::view::tempo_curve::CURVE)
                );
                assert_eq!(shape.stroke.width, 1.0);
                // Exact Bézier controls preserve bends even within a single screen pixel.
                let points = shape.points;
                assert!((points[0].x - (rect.left() + 2.0)).abs() < 0.01);
                assert!((points[1].x - (rect.right() - 2.0)).abs() < 0.01);
                assert!(points[0].x < points[2].x && points[2].x < points[1].x);
                for (point, tick) in points.iter().zip([960.0, 4000.0, 10000.0]) {
                    let expected = rect.top() + rect.height() * tick / 13920.0;
                    assert!((point.y - expected).abs() < 0.01);
                }
                assert!(output
                    .shapes
                    .iter()
                    .skip(index + 1)
                    .any(|s| { matches!(&s.shape, egui::Shape::Rect(r) if r.fill == DARK) }));
            }
        }
    }
}
