use super::*;
use cat_smf_editor::edit::tempo_curve::Point;

mod performance;

#[test]
fn narrow_bends_keep_their_height_while_zooming_and_following() {
    let curve = Curve {
        version: 1,
        points: vec![
            Point {
                tick: 0,
                bpm: 100.0,
            },
            Point {
                tick: 1000,
                bpm: 100.0,
            },
            Point {
                tick: 1010,
                bpm: 100.0,
            },
            Point {
                tick: 1020,
                bpm: 100.0,
            },
            Point {
                tick: 10000,
                bpm: 100.0,
            },
        ],
        controls: vec![
            Point {
                tick: 500,
                bpm: 100.0,
            },
            Point {
                tick: 1000,
                bpm: 200.0,
            },
            Point {
                tick: 1020,
                bpm: 40.0,
            },
            Point {
                tick: 6000,
                bpm: 100.0,
            },
        ],
    };
    curve.validate().unwrap();
    for scale in [0.2, 0.04, 0.008] {
        for offset in [0.0, 0.7, 1.6, 2.9, 1005.0 * scale] {
            for axis in [
                Axis {
                    low: 0.0,
                    high: 200.0,
                },
                Axis {
                    low: 90.0,
                    high: 110.0,
                },
            ] {
                let roll = Roll {
                    rect: egui::Rect::from_min_size(
                        egui::pos2(-offset, 10.0),
                        egui::vec2(2000.0, 300.0),
                    ),
                    scale,
                    axis,
                };
                let visible =
                    egui::Rect::from_min_max(egui::pos2(0.0, 10.0), egui::pos2(400.0, 310.0));
                let points = line(&roll, &curve.points, &curve.controls, visible);
                // Knot and extremum heights must survive even at subpixel widths.
                for (i, pair) in curve.points.windows(2).enumerate() {
                    let c = curve.controls[i];
                    let t = (pair[0].bpm - c.bpm) / (pair[0].bpm - 2.0 * c.bpm + pair[1].bpm);
                    for t in [0.0, t, 1.0]
                        .into_iter()
                        .filter(|t| t.is_finite() && *t >= 0.0 && *t <= 1.0)
                    {
                        let p = sample(
                            [
                                [pair[0].tick as f64, pair[0].bpm],
                                [c.tick as f64, c.bpm],
                                [pair[1].tick as f64, pair[1].bpm],
                            ],
                            t,
                        );
                        let expected = roll.pinned(roll.pos(p[0], p[1]));
                        if visible.x_range().contains(expected.x) {
                            assert!(
                                points.iter().any(|p| p.distance(expected) < 0.001),
                                "missing knot/extremum: {expected:?}"
                            );
                        }
                    }
                }
                for i in 0..=400 {
                    let tick = 1000.0 + i as f64 / 20.0;
                    let expected = roll.pinned(roll.pos(tick, curve.bpm_at(tick)));
                    if !visible.x_range().contains(expected.x) {
                        continue;
                    }
                    // Screen-distance error is the useful metric after horizontal
                    // compression; Y-at-the-same-X over-tessellates vertical lines.
                    let distance = points
                        .windows(2)
                        .map(|pair| {
                            let d = pair[1] - pair[0];
                            let t = ((expected - pair[0]).dot(d) / d.length_sq()).clamp(0.0, 1.0);
                            (pair[0] + t * d).distance(expected)
                        })
                        .fold(f32::INFINITY, f32::min);
                    assert!(
                        distance < 0.15,
                        "scale={scale} offset={offset} tick={tick}: distance={distance}"
                    );
                }
            }
        }
    }
}
