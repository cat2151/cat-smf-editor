//! Geometry and point operations shared by tempo and plugin parameter curves.
pub trait ScalarPoint: Copy {
    fn tick(self) -> u64;
    fn value(self) -> f64;
    fn new(tick: u64, value: f64) -> Self;
}

pub fn value_at<P: ScalarPoint>(points: &[P], controls: &[P], tick: f64) -> Option<f64> {
    if tick.is_nan() {
        return None;
    }
    let (first, last) = (*points.first()?, *points.last()?);
    if tick <= first.tick() as f64 {
        return Some(first.value());
    }
    if tick >= last.tick() as f64 {
        return Some(last.value());
    }
    let i = points.partition_point(|p| p.tick() as f64 <= tick) - 1;
    let (a, b, c) = (points[i], points[i + 1], *controls.get(i)?);
    let bezier = |s: f64, a: f64, c: f64, b: f64| {
        (1.0 - s) * (1.0 - s) * a + 2.0 * s * (1.0 - s) * c + s * s * b
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..60 {
        let middle = (low + high) / 2.0;
        if bezier(middle, a.tick() as f64, c.tick() as f64, b.tick() as f64) < tick {
            low = middle;
        } else {
            high = middle;
        }
    }
    // A quadratic Bezier stays within its control values' convex hull. Floating
    // point sums can otherwise turn a constant 1.0 into 1.0000000000000002,
    // which a native parameter correctly rejects as out of range.
    let minimum = a.value().min(b.value()).min(c.value());
    let maximum = a.value().max(b.value()).max(c.value());
    Some(bezier((low + high) / 2.0, a.value(), c.value(), b.value()).clamp(minimum, maximum))
}

pub fn midpoint<P: ScalarPoint>(a: P, b: P) -> P {
    P::new(
        a.tick() + (b.tick() - a.tick()) / 2,
        a.value() / 2.0 + b.value() / 2.0,
    )
}

/// Preserve controls only for unchanged adjacent endpoints; split/join gets a line.
pub fn rebuild_controls<P: ScalarPoint>(old: &[P], controls: &[P], points: &[P]) -> Vec<P> {
    points
        .windows(2)
        .map(|pair| {
            old.windows(2)
                .position(|previous| {
                    previous[0].tick() == pair[0].tick() && previous[1].tick() == pair[1].tick()
                })
                .and_then(|i| controls.get(i).copied())
                .unwrap_or_else(|| midpoint(pair[0], pair[1]))
        })
        .collect()
}

pub fn insert<P: ScalarPoint>(
    points: &mut Vec<P>,
    controls: &mut Vec<P>,
    point: P,
) -> Result<usize, String> {
    let index = points.partition_point(|p| p.tick() < point.tick());
    if points.get(index).is_some_and(|p| p.tick() == point.tick()) {
        return Err("A curve point already exists at this tick".into());
    }
    let old = points.clone();
    points.insert(index, point);
    *controls = rebuild_controls(&old, controls, points);
    Ok(index)
}

pub fn remove<P: ScalarPoint>(
    points: &mut Vec<P>,
    controls: &mut Vec<P>,
    index: usize,
) -> Result<(), String> {
    if index >= points.len() {
        return Err("No such curve point".into());
    }
    let old = points.clone();
    points.remove(index);
    *controls = rebuild_controls(&old, controls, points);
    Ok(())
}

pub fn move_point<P: ScalarPoint>(
    points: &mut [P],
    controls: &mut [P],
    index: usize,
    tick: u64,
    value: f64,
) -> Result<(), String> {
    if index >= points.len() || !value.is_finite() {
        return Err("Invalid curve point".into());
    }
    let min = index
        .checked_sub(1)
        .map_or(0, |i| points[i].tick().saturating_add(1));
    let max = points
        .get(index + 1)
        .map_or(u64::MAX, |p| p.tick().saturating_sub(1));
    if tick < min || tick > max {
        return Err("Curve points must remain in tick order".into());
    }
    points[index] = P::new(tick, value);
    for (pair, control) in points.windows(2).zip(controls) {
        *control = P::new(
            control.tick().clamp(pair[0].tick(), pair[1].tick()),
            control.value(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::parameter_curve::Point;

    #[test]
    fn interpolation_never_exceeds_the_control_value_hull_at_native_boundaries() {
        for value in [0.0, 1.0, -1.0, 0.3386160731315613] {
            let points = [Point { tick: 0, value }, Point { tick: 3835, value }];
            let controls = [Point { tick: 2000, value }];
            for i in 0..10000 {
                assert_eq!(
                    value_at(&points, &controls, f64::from(i) * 0.3835),
                    Some(value)
                );
            }
            assert_eq!(value_at(&points, &controls, 0.02), Some(value));
        }
    }
}
