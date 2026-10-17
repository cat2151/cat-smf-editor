//! Screen-space flattening, retaining every knot and vertical extremum at any zoom.
use super::*;

const TOLERANCE: f32 = 0.1;
type Position = [f64; 2];

pub(super) fn line(roll: &Roll, curve: &Curve, visible: egui::Rect) -> Vec<egui::Pos2> {
    let left = f64::from((visible.left() - roll.rect.left()).max(0.0));
    let right = f64::from((visible.right() - roll.rect.left()).max(left as f32));
    let scale = f64::from(roll.scale);
    // Local coordinates avoid losing precision to large tick positions/scroll offsets.
    let pos = |tick: f64, bpm: f64| {
        [
            tick * scale - left,
            (roll.axis.high - bpm) / (roll.axis.high - roll.axis.low)
                * f64::from(roll.rect.height()),
        ]
    };
    let mut raw = vec![pos(left / scale, curve.bpm_at(left / scale))];
    let first = curve
        .points
        .partition_point(|p| (p.tick as f64) < left / scale)
        .saturating_sub(1);
    let end = curve
        .points
        .partition_point(|p| p.tick as f64 <= right / scale)
        .min(curve.controls.len());
    for i in first..end {
        let a = curve.points[i];
        let c = curve.controls[i];
        let b = curve.points[i + 1];
        flatten(
            [
                pos(a.tick as f64, a.bpm),
                pos(c.tick as f64, c.bpm),
                pos(b.tick as f64, b.bpm),
            ],
            right - left,
            &mut raw,
        );
    }
    raw.push(pos(right / scale, curve.bpm_at(right / scale)));
    let screen = |p: Position| {
        roll.pinned(egui::pos2(
            (f64::from(roll.rect.left()) + left + p[0]) as f32,
            (f64::from(roll.rect.top()) + p[1]) as f32,
        ))
    };
    let mut line = vec![screen(raw[0])];
    for pair in raw.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        // Insert edge crossings before pinning, rather than moving control points.
        let mut crossings =
            [0.0, f64::from(roll.rect.height())].map(|edge| (edge - a[1]) / (b[1] - a[1]));
        crossings.sort_by(f64::total_cmp);
        for t in crossings.into_iter().filter(|t| *t > 0.0 && *t < 1.0) {
            line.push(screen(lerp(a, b, t)));
        }
        line.push(screen(b));
    }
    line.dedup();
    line
}

fn lerp(a: Position, b: Position, t: f64) -> Position {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn sample([a, c, b]: [Position; 3], t: f64) -> Position {
    lerp(lerp(a, c, t), lerp(c, b, t), t)
}

fn at_x(p: [Position; 3], x: f64) -> f64 {
    if x <= p[0][0] {
        return 0.0;
    }
    if x >= p[2][0] {
        return 1.0;
    }
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..48 {
        let t = (low + high) / 2.0;
        if sample(p, t)[0] < x {
            low = t;
        } else {
            high = t;
        }
    }
    (low + high) / 2.0
}

fn flatten(p: [Position; 3], width: f64, out: &mut Vec<Position>) {
    let start = at_x(p, 0.0);
    let end = at_x(p, width);
    let [a, c, b] = p;
    let extremum = (a[1] - c[1]) / (a[1] - 2.0 * c[1] + b[1]);
    let mut cuts = vec![start];
    if extremum > start && extremum < end {
        cuts.push(extremum);
    }
    cuts.push(end);
    for range in cuts.windows(2) {
        let (s, t) = (range[0], range[1]);
        let from = sample(p, s);
        let to = sample(p, t);
        let tangent = lerp([c[0] - a[0], c[1] - a[1]], [b[0] - c[0], b[1] - c[1]], s);
        let control = [
            from[0] + (t - s) * tangent[0],
            from[1] + (t - s) * tangent[1],
        ];
        let shape = egui::epaint::QuadraticBezierShape::from_points_stroke(
            [from, control, to].map(|v| egui::pos2(v[0] as f32, v[1] as f32)),
            false,
            Color32::TRANSPARENT,
            egui::Stroke::NONE,
        );
        out.push(from);
        shape.for_each_flattened_with_t(TOLERANCE, &mut |v, t| {
            if t < 1.0 {
                out.push([f64::from(v.x).clamp(from[0], to[0]), f64::from(v.y)]);
            }
        });
        // Keep exact extrema even if the library considers a tiny piece a point.
        out.push(to);
    }
}

#[cfg(test)]
mod tests;
