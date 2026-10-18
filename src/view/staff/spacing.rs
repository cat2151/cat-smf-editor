//! Engraved x positions: each measure keeps its width, and the onsets inside it share that
//! width by the square root of the time to the next onset, so rests shrink and runs widen
//! without making every note equally spaced.
use crate::app::Loaded;

pub(super) struct Spacing {
    /// x of each measure start and each onset inside a measure, ascending by tick.
    points: Vec<(f64, f32)>,
    /// Measure starts, ascending.
    bars: Vec<f64>,
    scale: f32,
    /// Room between a barline and its measure's first onset.
    pad: f32,
}

/// Measure start ticks of `track`'s meter up to past `end`.
pub(super) fn bars(loaded: &Loaded, track: usize, end: u64, ppqn: u16) -> Vec<f64> {
    loaded.time.map(track).map_or_else(Vec::new, |m| {
        m.beat_runs(end + u64::from(ppqn) * 16)
            .iter()
            .flat_map(|run| {
                (0..run.count)
                    .filter(|&i| run.position(i).beat == 1)
                    .map(|i| run.tick as f64 + (i * run.step) as f64 / run.denominator as f64)
            })
            .collect()
    })
}

impl Spacing {
    /// `onsets` ascending without duplicates.
    pub fn new(bars: Vec<f64>, onsets: &[u64], scale: f32, pad: f32) -> Self {
        let mut points = Vec::new();
        let mut rest = onsets.iter().map(|&t| t as f64).peekable();
        for pair in bars.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let mut ticks = vec![start];
            while let Some(tick) = rest.next_if(|&t| t < end) {
                if tick > start {
                    ticks.push(tick);
                }
            }
            if end <= start {
                continue;
            }
            ticks.push(end);
            let weights: Vec<f64> = ticks.windows(2).map(|w| (w[1] - w[0]).sqrt()).collect();
            let total: f64 = weights.iter().sum();
            let width = (end - start) * f64::from(scale);
            let mut x = start * f64::from(scale) + f64::from(pad);
            for (&tick, weight) in ticks.iter().zip(&weights) {
                points.push((tick, x as f32));
                x += width * weight / total;
            }
        }
        Self {
            points,
            bars,
            scale,
            pad,
        }
    }

    /// x of `tick` from the left edge of the score.
    pub fn x(&self, tick: u64) -> f32 {
        let tick = tick as f64;
        let i = self.points.partition_point(|p| p.0 <= tick);
        match (i.checked_sub(1).map(|j| self.points[j]), self.points.get(i)) {
            (Some(a), Some(b)) => a.1 + (b.1 - a.1) * ((tick - a.0) / (b.0 - a.0)) as f32,
            _ => tick as f32 * self.scale + self.pad,
        }
    }

    /// Inverse of [`Self::x`].
    pub fn tick(&self, x: f32) -> u64 {
        let i = self.points.partition_point(|p| p.1 <= x);
        let tick = match (i.checked_sub(1).map(|j| self.points[j]), self.points.get(i)) {
            (Some(a), Some(b)) => a.0 + (b.0 - a.0) * f64::from((x - a.1) / (b.1 - a.1)),
            _ => f64::from((x - self.pad) / self.scale),
        };
        tick.round().max(0.0) as u64
    }

    /// The measure and quarter note within it that hold `tick`.
    pub fn beat(&self, tick: u64, ppqn: u16) -> (usize, u64) {
        let tick = tick as f64;
        let i = self.bars.partition_point(|&b| b <= tick);
        let bar = i.checked_sub(1).map_or(0.0, |j| self.bars[j]);
        (i, ((tick - bar) / f64::from(ppqn.max(1))) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_measure_keeps_its_width_and_compresses_long_rests() {
        // 4/4 at 480 ppqn: four sixteenths, then a rest to the barline.
        let spacing = Spacing::new(vec![0.0, 1920.0, 3840.0], &[0, 120, 240, 360], 0.1, 5.0);
        assert_eq!(spacing.x(0), 5.0);
        assert!((spacing.x(1920) - 197.0).abs() < 1e-3);
        let sixteenth = spacing.x(120) - spacing.x(0);
        assert!(
            sixteenth > 12.0 * 1.5,
            "short notes get more than linear room"
        );
        let rest = spacing.x(1920) - spacing.x(360);
        assert!(
            rest > sixteenth * 2.0,
            "a long rest still takes more room than a note"
        );
        // An empty measure stays linear.
        assert!((spacing.x(2880) - (288.0 + 5.0)).abs() < 1e-3);
    }

    #[test]
    fn tick_inverts_x() {
        let spacing = Spacing::new(vec![0.0, 1920.0, 3840.0], &[0, 120, 240, 360], 0.1, 5.0);
        for tick in [0, 60, 120, 360, 1000, 1920, 2880, 5000] {
            assert_eq!(spacing.tick(spacing.x(tick)), tick);
        }
        assert_eq!(spacing.tick(0.0), 0);
    }

    #[test]
    fn beats_count_quarters_from_the_barline() {
        let spacing = Spacing::new(vec![0.0, 1440.0], &[], 0.1, 0.0);
        assert_eq!(spacing.beat(479, 480), (1, 0));
        assert_eq!(spacing.beat(480, 480), (1, 1));
        assert_eq!(spacing.beat(1440, 480), (2, 0));
    }
}
