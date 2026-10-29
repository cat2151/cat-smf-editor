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
