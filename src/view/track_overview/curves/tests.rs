use super::*;
use cat_smf_editor::edit::parameter_curve::{Point, Target, ValueSpace};

fn point(tick: u64, value: f64) -> Point {
    Point { tick, value }
}

#[test]
fn a_narrow_bend_keeps_its_height_while_scrolling() {
    let curve = Curve {
        version: 1,
        target: Target {
            group: 1,
            layer: 1,
            plugin_format: "VST3".into(),
            plugin_id: "plugin".into(),
            parameter_id: "1".into(),
        },
        value_space: ValueSpace::Normalized,
        minimum: 0.0,
        maximum: 1.0,
        name: "Cutoff".into(),
        path: String::new(),
        points: vec![
            point(0, 0.5),
            point(1000, 0.5),
            point(1020, 0.5),
            point(10000, 0.5),
        ],
        controls: vec![point(500, 0.5), point(1010, 1.0), point(6000, 0.5)],
    };
    // 0.04 px per tick: the 20-tick bend is narrower than a pixel.
    let scale = 0.04;
    let clip = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 104.0));
    let peaks: Vec<f32> = [0.0, 0.3, 0.5, 0.7, 1.1]
        .into_iter()
        .map(|offset| {
            let rect =
                egui::Rect::from_min_size(egui::pos2(-offset, 0.0), egui::vec2(400.0, 104.0));
            curve_line(rect, clip, scale, &curve)
                .iter()
                .map(|p| p.y)
                .fold(f32::INFINITY, f32::min)
        })
        .collect();
    for peak in &peaks {
        assert!((peak - peaks[0]).abs() < 0.01, "{peaks:?}");
    }
}
