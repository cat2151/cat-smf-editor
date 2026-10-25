//! Opt-in CPU measurement; does not measure GPU submission/presentation or the rest of the UI.
use super::*;
use std::{hint::black_box, time::Instant};

#[test]
#[ignore = "diagnostic: run with --release --ignored --nocapture"]
fn curve_path_cpu_cost() {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    for segments in [100, 1000, 10000] {
        let curve = Curve {
            version: 1,
            points: (0..=segments)
                .map(|i| Point {
                    tick: i * 480,
                    bpm: 100.0,
                })
                .collect(),
            controls: (0..segments)
                .map(|i| Point {
                    tick: i * 480 + 240,
                    bpm: if i % 2 == 0 { 180.0 } else { 40.0 },
                })
                .collect(),
        };
        for visible_segments in [10, 100, segments] {
            let scale = 1400.0 / (visible_segments * 480) as f32;
            let visible = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 300.0));
            let mut build = 0.0;
            let mut tessellate = 0.0;
            let mut vertices = 0;
            let mut count = 0;
            for frame in 0..60 {
                let roll = Roll {
                    rect: egui::Rect::from_min_size(
                        egui::pos2(-(frame as f32 * 0.37), 0.0),
                        egui::vec2(segments as f32 * 480.0 * scale, 300.0),
                    ),
                    scale,
                    axis: Axis {
                        low: 40.0,
                        high: 180.0,
                    },
                };
                let start = Instant::now();
                let points = line(
                    black_box(&roll),
                    black_box(&curve.points),
                    &curve.controls,
                    visible,
                );
                build += start.elapsed().as_secs_f64();
                count = points.len();
                let shapes = vec![egui::epaint::ClippedShape {
                    clip_rect: visible,
                    shape: egui::Shape::line(points, egui::Stroke::new(2.0_f32, CURVE)),
                }];
                let start = Instant::now();
                let primitives = ctx.tessellate(shapes, 1.0);
                tessellate += start.elapsed().as_secs_f64();
                vertices = primitives
                    .iter()
                    .map(|p| match &p.primitive {
                        egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                        _ => 0,
                    })
                    .sum::<usize>();
                black_box(primitives);
            }
            println!("segments={segments} visible={visible_segments} path={count} vertices={vertices} build_ms={:.3} tessellate_ms={:.3}",
                build * 1000.0 / 60.0, tessellate * 1000.0 / 60.0);
        }
    }
}
