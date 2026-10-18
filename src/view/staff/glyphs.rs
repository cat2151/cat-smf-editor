//! Clefs, accidentals and notes drawn from curves, so the score needs no font or image asset.
use eframe::egui::{self, Color32, Pos2, Shape, Stroke};

/// Draws in staff-gap units with y up, around `origin`.
pub(super) struct Pen<'a> {
    pub painter: &'a egui::Painter,
    pub origin: Pos2,
    pub gap: f32,
    pub color: Color32,
}

type Curve = [(f32, f32); 4];

impl Pen<'_> {
    fn at(&self, (x, y): (f32, f32)) -> Pos2 {
        self.origin + egui::vec2(x, -y) * self.gap
    }

    fn stroke(&self, width: f32) -> Stroke {
        Stroke::new((width * self.gap).max(1.0), self.color)
    }

    fn line(&self, from: (f32, f32), to: (f32, f32), width: f32) {
        self.painter
            .line_segment([self.at(from), self.at(to)], self.stroke(width));
    }

    fn dot(&self, center: (f32, f32), radius: f32) {
        self.painter
            .circle_filled(self.at(center), radius * self.gap, self.color);
    }

    /// Consecutive cubic Béziers as one stroked path.
    fn curves(&self, curves: &[Curve], width: f32) {
        let mut points = Vec::new();
        for curve in curves {
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let u = 1.0 - t;
                let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                let (x, y) = curve
                    .iter()
                    .zip(weights)
                    .fold((0.0, 0.0), |(x, y), (p, w)| (x + p.0 * w, y + p.1 * w));
                points.push(self.at((x, y)));
            }
        }
        self.painter.add(Shape::line(points, self.stroke(width)));
    }

    /// Treble clef with its curl on the G line at the origin.
    pub fn treble_clef(&self) {
        self.curves(
            &[
                [(0.15, 0.1), (0.1, 0.65), (1.0, 0.65), (1.0, 0.0)],
                [(1.0, 0.0), (1.0, -0.8), (-0.75, -0.85), (-0.75, 0.15)],
                [(-0.75, 0.15), (-0.75, 1.1), (0.65, 1.7), (0.6, 2.8)],
                [(0.6, 2.8), (0.55, 3.6), (0.25, 4.2), (0.05, 4.1)],
                [(0.05, 4.1), (-0.3, 3.9), (-0.35, 3.0), (-0.05, 2.3)],
                [(-0.05, 2.3), (0.2, 1.3), (0.45, -1.0), (0.4, -2.2)],
                [(0.4, -2.2), (0.35, -2.85), (-0.45, -2.85), (-0.45, -2.25)],
            ],
            0.16,
        );
        self.dot((-0.2, -2.2), 0.28);
    }

    /// Bass clef with its head on the F line at the origin.
    pub fn bass_clef(&self) {
        self.dot((0.25, 0.0), 0.3);
        self.curves(
            &[
                [(0.05, 0.05), (0.1, 0.85), (1.5, 0.95), (1.5, 0.0)],
                [(1.5, 0.0), (1.5, -1.0), (0.8, -1.9), (-0.05, -2.5)],
            ],
            0.2,
        );
        self.dot((2.0, 0.5), 0.14);
        self.dot((2.0, -0.5), 0.14);
    }

    /// Sharp, natural or flat centered on the origin's line or space.
    pub fn accidental(&self, alter: i8) {
        match alter {
            1 => {
                self.line((-0.22, -1.25), (-0.22, 1.1), 0.1);
                self.line((0.22, -1.1), (0.22, 1.25), 0.1);
                self.line((-0.5, -0.5), (0.5, -0.25), 0.28);
                self.line((-0.5, 0.25), (0.5, 0.5), 0.28);
            }
            0 => {
                self.line((-0.25, 1.3), (-0.25, -0.5), 0.1);
                self.line((0.25, 0.5), (0.25, -1.3), 0.1);
                self.line((-0.25, -0.5), (0.25, -0.3), 0.25);
                self.line((-0.25, 0.3), (0.25, 0.5), 0.25);
            }
            _ => {
                self.line((-0.3, 2.0), (-0.3, -0.5), 0.1);
                self.curves(
                    &[[(-0.3, 0.2), (0.5, 0.9), (0.6, -0.05), (-0.3, -0.5)]],
                    0.18,
                );
            }
        }
    }

    /// Note head with its left edge at the origin; `value` is from `notation::value`.
    /// Returns the head width in pixels.
    pub fn head(&self, value: i32) -> f32 {
        let hollow = value >= 1;
        let (a, b, tilt) = if value >= 2 {
            (0.75, 0.45, 0.0_f32)
        } else {
            (0.62, 0.42, 0.35_f32)
        };
        let points: Vec<Pos2> = (0..20)
            .map(|i| {
                let t = i as f32 / 20.0 * std::f32::consts::TAU;
                let (x, y) = (a * t.cos(), b * t.sin());
                self.at((
                    a + x * tilt.cos() - y * tilt.sin(),
                    x * tilt.sin() + y * tilt.cos(),
                ))
            })
            .collect();
        if hollow {
            self.painter
                .add(Shape::closed_line(points, self.stroke(0.15)));
        } else {
            self.painter
                .add(Shape::convex_polygon(points, self.color, Stroke::NONE));
        }
        2.0 * a * self.gap
    }

    /// Note head with its stem and flags, as `head` places it.
    pub fn note(&self, value: i32, stem_up: bool) -> f32 {
        let width = self.head(value);
        if value < 2 {
            let (x, direction) = (stem_x(stem_up), if stem_up { 1.0 } else { -1.0 });
            let tip = 3.5 * direction;
            self.line((x, 0.2 * direction), (x, tip), STEM);
            for flag in 0..(-value).max(0) {
                let y = tip - 0.8 * flag as f32 * direction;
                self.curves(
                    &[[
                        (x, y),
                        (x + 0.15, y - 1.0 * direction),
                        (x + 1.1, y - 1.2 * direction),
                        (x + 0.8, y - 2.3 * direction),
                    ]],
                    0.18,
                );
            }
        }
        width
    }
}

/// Stem width in gaps.
pub(super) const STEM: f32 = 0.12;

/// Stem x in gaps from a stemmed head's left edge: its right side when up, left when down.
pub(super) fn stem_x(up: bool) -> f32 {
    if up {
        2.0 * 0.62 - 0.06
    } else {
        0.06
    }
}
