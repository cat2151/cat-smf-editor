//! Equal-width measures, interpolated using their actual meters.
use crate::view::timeline_repeat::TickAxis;
use cat_smf_editor::score::{musical_time::MeterMap, same_measure};
pub(crate) struct Cells<'a> {
    pub map: &'a MeterMap,
    pub stride: f32,
    pub label: f32,
}
impl TickAxis for Cells<'_> {
    fn x(&self, tick: u64) -> f32 {
        let measure = self.map.position(tick).measure;
        let Ok((start, end)) = same_measure::bounds(self.map, measure) else {
            return self.label;
        };
        self.label
            + ((measure - 1) as f32 + (tick - start) as f32 / (end - start) as f32) * self.stride
    }
    fn tick(&self, x: f32) -> u64 {
        let cell = ((x - self.label) / self.stride).max(0.0);
        let Ok((start, end)) = same_measure::bounds(self.map, cell.floor() as u128 + 1) else {
            return 0;
        };
        start.saturating_add(((end - start) as f32 * cell.fract()).round() as u64)
    }
}

/// Scene-local meters; never substitute the Song conductor's meter map.
pub(crate) struct SceneCells<'a> {
    pub boundaries: &'a [u64],
    pub stride: f32,
    pub label: f32,
}
impl TickAxis for SceneCells<'_> {
    fn x(&self, tick: u64) -> f32 {
        let i = self
            .boundaries
            .partition_point(|&t| t <= tick)
            .saturating_sub(1)
            .min(self.boundaries.len().saturating_sub(2));
        let start = self.boundaries[i];
        let length = self.boundaries[i + 1] - start;
        self.label
            + (i as f32 + tick.saturating_sub(start) as f32 / length.max(1) as f32) * self.stride
    }
    fn tick(&self, x: f32) -> u64 {
        let cell = ((x - self.label) / self.stride).max(0.0);
        let i = (cell.floor() as usize).min(self.boundaries.len().saturating_sub(2));
        let fraction = cell - i as f32;
        self.boundaries[i]
            + ((self.boundaries[i + 1] - self.boundaries[i]) as f32 * fraction).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_smf_editor::model::{Event, Music, Track};

    #[test]
    fn song_cells_follow_changing_meters_and_exclude_the_label_from_tick_zero() {
        let music = Music {
            format: 0,
            division: 480,
            tracks: vec![Track {
                events: [(0, 4, 2), (1920, 3, 2), (3360, 5, 3)]
                    .map(|(tick, numerator, exponent)| Event {
                        tick,
                        delta: 0,
                        status: 0xff,
                        meta_type: 0x58,
                        data: vec![numerator, exponent, 24, 8],
                    })
                    .to_vec(),
            }],
        };
        let time = cat_smf_editor::score::musical_time::MusicalTime::new(&music);
        let axis = Cells {
            map: time.map(0).unwrap(),
            stride: 100.0,
            label: 24.0,
        };
        for (tick, x) in [
            (0, 24.0),
            (1920, 124.0),
            (2640, 174.0),
            (3360, 224.0),
            (4560, 324.0),
        ] {
            assert!((axis.x(tick) - x).abs() < 0.01);
            assert_eq!(axis.tick(x), tick);
        }
    }

    #[test]
    fn scene_cells_use_intrinsic_meter_lengths() {
        let axis = SceneCells {
            boundaries: &[0, 1440, 3360],
            stride: 100.0,
            label: 24.0,
        };
        assert_eq!(axis.x(1440), 124.0);
        assert_eq!(axis.x(2400), 174.0);
        assert_eq!(axis.tick(174.0), 2400);
        assert_eq!(axis.tick(224.0), 3360);
        assert_eq!(axis.tick(0.0), 0);
    }
}
