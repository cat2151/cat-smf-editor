//! Adding, removing and moving the points of a curve so that it stays valid.
use super::Target;
use cat_smf_editor::edit::tempo_curve::{bpm_range, Curve, Point};

/// The control point that makes the segment from `a` to `b` straight.
fn midpoint(a: Point, b: Point) -> Point {
    Point {
        tick: a.tick + (b.tick - a.tick) / 2,
        bpm: (a.bpm + b.bpm) / 2.0,
    }
}

fn clamp_bpm(bpm: f64) -> f64 {
    bpm.clamp(*bpm_range().start(), *bpm_range().end())
}

fn tick_of(tick: f64) -> u64 {
    tick.round().max(0.0) as u64
}

/// Controls for `points`: segments touching `changed` become straight, and the others keep
/// the control of the old segment `old(j)` returns for new segment `j`.
fn controls(
    points: &[Point],
    changed: impl Fn(usize) -> bool,
    old: impl Fn(usize) -> Point,
) -> Vec<Point> {
    (0..points.len().saturating_sub(1))
        .map(|j| {
            if changed(j) {
                midpoint(points[j], points[j + 1])
            } else {
                old(j)
            }
        })
        .collect()
}

/// `curve` with a tempo point at (`tick`, `bpm`) whose segments are straight; a new curve
/// when there is none. `None` when a tempo point already sits at that tick.
pub(crate) fn add(curve: Option<&Curve>, tick: f64, bpm: f64) -> Option<Curve> {
    let point = Point {
        tick: tick_of(tick),
        bpm: clamp_bpm(bpm),
    };
    let Some(curve) = curve else {
        return Some(Curve {
            version: 1,
            points: vec![point],
            controls: Vec::new(),
        });
    };
    let i = curve.points.partition_point(|p| p.tick < point.tick);
    if curve.points.get(i).is_some_and(|p| p.tick == point.tick) {
        return None;
    }
    let mut points = curve.points.clone();
    points.insert(i, point);
    let controls = controls(
        &points,
        |j| j + 1 == i || j == i,
        |j| curve.controls[if j < i { j } else { j - 1 }],
    );
    Some(Curve {
        points,
        controls,
        ..curve.clone()
    })
}

/// `curve` without tempo point `i`; its neighbors join in one straight segment.
/// `None` when it was the last point.
pub(crate) fn remove(curve: &Curve, i: usize) -> Option<Curve> {
    if curve.points.len() <= 1 {
        return None;
    }
    let mut points = curve.points.clone();
    points.remove(i);
    let controls = controls(
        &points,
        |j| j + 1 == i,
        |j| curve.controls[if j + 1 < i { j } else { j + 1 }],
    );
    Some(Curve {
        points,
        controls,
        ..curve.clone()
    })
}

/// `curve` with point `i` of `target`'s kind moved toward (`tick`, `bpm`).
/// A tempo point stays strictly between its neighbors, and the controls beside it are pulled
/// into their shortened segments; a control point stays within its segment.
pub(crate) fn moved(curve: &Curve, target: Target, i: usize, tick: f64, bpm: f64) -> Curve {
    let mut curve = curve.clone();
    let (tick, bpm) = (tick_of(tick), clamp_bpm(bpm));
    match target {
        Target::Point => {
            let low = if i == 0 {
                0
            } else {
                curve.points[i - 1].tick + 1
            };
            let high = curve.points.get(i + 1).map_or(u64::MAX, |p| p.tick - 1);
            let tick = tick.clamp(low, high);
            curve.points[i] = Point { tick, bpm };
            if i > 0 {
                let c = &mut curve.controls[i - 1];
                c.tick = c.tick.clamp(curve.points[i - 1].tick, tick);
            }
            if let Some(c) = curve.controls.get_mut(i) {
                c.tick = c.tick.clamp(tick, curve.points[i + 1].tick);
            }
        }
        Target::Control => {
            let (low, high) = (curve.points[i].tick, curve.points[i + 1].tick);
            curve.controls[i] = Point {
                tick: tick.clamp(low, high),
                bpm,
            };
        }
    }
    curve
}
