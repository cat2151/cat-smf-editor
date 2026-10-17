//! Read-only tempo layers: opening the roll never initializes or writes metadata.
use super::*;
use crate::view::timeline_minimap::tempos::Tempos;
use cat_smf_editor::edit::{reference_tempo, tempo_curve};

pub(super) struct Data {
    events: Vec<(u64, f64)>,
    curve: Option<tempo_curve::Curve>,
    references: Vec<reference_tempo::Reference>,
    pub error: Option<String>,
}

impl Data {
    pub fn read(loaded: &Loaded, track: usize) -> Self {
        let source = if loaded.music.format == 1 { 0 } else { track };
        let events = loaded.music.tracks[source]
            .events
            .iter()
            .filter(|e| e.status == 0xff && e.meta_type == 0x51 && e.data.len() == 3)
            .map(|e| {
                (
                    e.tick,
                    60_000_000.0
                        / f64::from(
                            u32::from_be_bytes([0, e.data[0], e.data[1], e.data[2]]).max(1),
                        ),
                )
            })
            .collect();
        let curve = tempo_curve::read(&loaded.music);
        let references = reference_tempo::read(&loaded.music);
        let error = curve.as_ref().err().or(references.as_ref().err()).cloned();
        Self {
            events,
            curve: curve.unwrap_or(None),
            references: references.unwrap_or_default(),
            error,
        }
    }

    pub fn extend_timeline(&self, loaded: &Loaded, meter: usize, layout: &mut Layout) {
        let end = self
            .curve
            .iter()
            .flat_map(|c| c.points.iter().chain(&c.controls))
            .map(|p| p.tick)
            .chain(self.references.iter().map(|p| p.tick))
            .max()
            .unwrap_or(0);
        let width = end as f32 * layout.scale + PX_PER_QUARTER;
        if width > layout.size.x {
            layout.size.x = width;
            layout.beats = beats_of(loaded, meter, end, layout.scale);
        }
    }

    pub fn layers(&self) -> Tempos<'_> {
        Tempos {
            events: &self.events,
            curve: self.curve.as_ref(),
            references: &self.references,
        }
    }
}
