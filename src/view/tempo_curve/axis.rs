use cat_smf_editor::edit::tempo_curve;

/// The BPM shown at the top and bottom of the roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Axis {
    pub(crate) low: f64,
    pub(crate) high: f64,
}

impl Axis {
    /// Exact song bounds. A constant tempo gets a small nonzero range for editing.
    pub(crate) fn new(bpms: impl Iterator<Item = f64>) -> Self {
        let (low, high) = bpms.fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), b| {
            (l.min(b), h.max(b))
        });
        let (low, high) = if low.is_finite() {
            (low, high)
        } else {
            (120.0, 120.0)
        };
        if low == high {
            Self {
                low: (low - 1.0).max(*tempo_curve::bpm_range().start()),
                high: (high + 1.0).min(*tempo_curve::bpm_range().end()),
            }
        } else {
            Self { low, high }
        }
    }

    pub(crate) fn y(&self, bpm: f64, height: f32) -> f32 {
        ((self.high - bpm) / (self.high - self.low)) as f32 * height
    }
}
