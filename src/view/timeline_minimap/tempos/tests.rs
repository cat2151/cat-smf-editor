use super::*;
use cat_smf_editor::edit::tempo_curve::Point;

fn point(tick: u64, bpm: f64) -> Point {
    Point { tick, bpm }
}

#[test]
fn a_narrow_bend_keeps_its_height_while_scrolling() {
    let curve = Curve {
        version: 1,
        points: vec![
            point(0, 100.0),
            point(1000, 100.0),
            point(1020, 100.0),
            point(10000, 100.0),
        ],
        controls: vec![point(500, 100.0), point(1010, 200.0), point(6000, 100.0)],
    };
    curve.validate().unwrap();
    let axis = Axis {
        low: 0.0,
        high: 200.0,
    };
    let end = 10000.0;
    // 0.04 px per tick: the 20-tick bend is narrower than a pixel.
    let width = 400.0;
    let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 104.0));
    let peaks: Vec<f32> = [0.0, 0.3, 0.5, 0.7, 1.1]
        .into_iter()
        .map(|offset| {
            let rect =
                egui::Rect::from_min_size(egui::pos2(-offset, 0.0), egui::vec2(width, 104.0));
            curve_line(rect, clip, end, axis, &curve)
                .iter()
                .map(|p| p.y)
                .fold(f32::INFINITY, f32::min)
        })
        .collect();
    for peak in &peaks {
        assert!((peak - peaks[0]).abs() < 0.01, "{peaks:?}");
    }
}
