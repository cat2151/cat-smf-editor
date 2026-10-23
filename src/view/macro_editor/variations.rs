use super::*;

pub(super) fn restore(
    saved: &macro_values::Values,
    layer: u64,
    rows: &[Row],
) -> (Vec<Vec<Curve>>, usize) {
    if let Some(set) = saved.variations.iter().find(|s| s.layer == layer) {
        (set.snapshots.clone(), set.selected)
    } else {
        (vec![rows.iter().map(Row::saved).collect()], 0)
    }
}
impl Session {
    pub(super) fn randomize(&mut self, full_range: bool) {
        let saved = self.saved_variations();
        for row in &mut self.rows {
            let (minimum, maximum) = if full_range {
                (0.0, 1.0)
            } else {
                saved
                    .snapshots
                    .iter()
                    .filter_map(|snapshot| {
                        let curve = snapshot.iter().find(|c| c.target == row.curve.target)?;
                        let domain = curve.domain().ok()?;
                        let value = domain.value(curve.points.first()?.value).ok()?;
                        domain.position(value).ok().map(|p| p.get())
                    })
                    .fold((row.get(), row.get()), |(low, high), value| {
                        (low.min(value), high.max(value))
                    })
            };
            // Positions map uniformly to both unipolar and bipolar macro values.
            row.set(minimum + (maximum - minimum) * fastrand::f64());
            row.readback = None;
        }
        self.readback_at = None;
    }

    pub(super) fn saved_variations(&self) -> macro_values::Variations {
        let mut snapshots = self.variations.clone();
        if snapshots.is_empty() {
            snapshots.push(vec![]);
        }
        snapshots[self.variation] = self.rows.iter().map(Row::saved).collect();
        macro_values::Variations {
            layer: self.layer,
            selected: self.variation,
            snapshots,
        }
    }
    pub(super) fn switch_variation(&mut self, next: usize) {
        let saved = self.saved_variations();
        if next >= saved.snapshots.len() {
            return;
        }
        self.variations = saved.snapshots;
        self.variation = next;
        // Preserve the atomics already held by the audio callback.
        for row in &mut self.rows {
            if let Some(curve) = self.variations[next]
                .iter()
                .find(|c| c.target == row.curve.target)
            {
                row.set(curve.points[0].value);
                row.readback = None;
            }
        }
        self.readback_at = None;
    }
    pub(super) fn duplicate_variation(&mut self) {
        let saved = self.saved_variations();
        if saved.snapshots.len() >= 128 {
            return;
        }
        self.variations = saved.snapshots;
        self.variations
            .push(self.variations[self.variation].clone());
        self.variation = self.variations.len() - 1;
    }
}
