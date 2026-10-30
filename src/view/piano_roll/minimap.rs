//! Read-only tempo layers: opening the roll never initializes or writes metadata.
use super::*;
use crate::view::timeline_minimap::tempos::Tempos;
use cat_smf_editor::edit::{reference_tempo, tempo_curve};

pub(crate) struct Data {
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

    /// Beats per minute at `tick` from the tempo events.
    pub fn tempo_at(&self, tick: u64) -> Option<f64> {
        self.events
            .iter()
            .take_while(|(t, _)| *t <= tick)
            .last()
            .map(|(_, bpm)| *bpm)
    }

    pub fn layers(&self) -> Tempos<'_> {
        Tempos {
            events: &self.events,
            curve: self.curve.as_ref(),
            references: &self.references,
        }
    }
}

// Note rendering is deliberately outside the gesture and viewport controller.
pub(crate) fn draw(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &crate::view::piano_roll::Layout,
    end: f32,
) {
    for (note, color) in layout
        .references
        .iter()
        .map(|n| (n, crate::view::piano_roll::REFERENCE_COLOR))
        .chain(
            layout
                .notes
                .iter()
                .zip(&layout.colors)
                .map(|((_, n), &c)| (n, c)),
        )
    {
        let point = |p: egui::Pos2| {
            rect.min
                + egui::vec2(
                    p.x / end.max(1.0) * rect.width(),
                    p.y / layout.size.y.max(1.0) * rect.height(),
                )
        };
        let mut note = egui::Rect::from_min_max(point(note.min), point(note.max));
        note.max = note.max.max(note.min + egui::Vec2::splat(1.0));
        painter.rect_filled(note, 0.0, color.gamma_multiply(0.65));
    }
}

impl crate::view::timeline_controls::Viewport {
    pub fn minimap_on(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        layout: &Layout,
        tempos: crate::view::timeline_minimap::tempos::Tempos<'_>,
        playing: Option<u64>,
        binding: &crate::view::timeline_controls::Binding<'_>,
    ) -> (egui::Rect, bool) {
        let (rect, area) = crate::view::timeline_minimap::area(area);
        let navigation = self
            .minimap_state
            .show_on(ui, rect, binding, playing, |painter, rect| {
                draw(painter, rect, layout, layout.size.x);
                crate::view::timeline_minimap::tempos::draw(
                    painter,
                    rect,
                    layout.size.x / layout.scale,
                    tempos,
                );
            });
        if let Some(navigation) = &navigation {
            self.seek = navigation.seek.or(self.seek);
            let mut offset = self.pending.unwrap_or(self.scroll);
            offset.x = navigation.scroll;
            self.pending = Some(offset);
        }
        (area, navigation.is_some())
    }

    pub fn scale_layout(&self, layout: &mut Layout) {
        let zoom = self.zoom();
        layout.size = layout.size * zoom;
        layout.start *= zoom.x;
        layout.scale *= zoom.x;
        for (x, _) in &mut layout.beats {
            *x *= zoom.x;
        }
        for y in &mut layout.staff {
            *y *= zoom.y;
        }
        for note in layout
            .notes
            .iter_mut()
            .map(|(_, note)| note)
            .chain(&mut layout.references)
        {
            note.min = (note.min.to_vec2() * zoom).to_pos2();
            note.max = (note.max.to_vec2() * zoom).to_pos2();
        }
    }
}
