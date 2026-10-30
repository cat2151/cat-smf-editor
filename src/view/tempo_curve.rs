//! `r` on the conductor: the tempo curve and the conductor's tempo events over every track's notes.
use crate::view::timeline_follow::Follow;
use crate::{
    app::{App, Loaded},
    view::piano_roll::{draw_beats, layout_of, Layout, PX_PER_QUARTER},
};
use cat_smf_editor::edit::tempo_curve::{self, Curve};
use eframe::egui::{self, Color32};

/// Screen points between out-of-range curve direction marks.
const EDGE_MARKER_STEP: f32 = 32.0;
/// BPM between grid lines.
const GRID_STEP: f64 = 10.0;
const POINT_RADIUS: f32 = 5.0;
const CONTROL_SIDE: f32 = 8.0;
const HOVER_SCALE: f32 = 1.5;
/// Pointer distance within which a point is hovered.
const HIT: f32 = 8.0;
pub(crate) const GRID: Color32 = Color32::from_gray(60);
pub(crate) const GRID_LABEL: Color32 = Color32::from_gray(140);
pub(crate) const LABEL_FONT: egui::FontId = egui::FontId::monospace(10.0);
pub(crate) const CURVE: Color32 = Color32::from_rgb(0xa6, 0xe2, 0x2e);
pub(crate) const TEMPO: Color32 = Color32::from_rgb(0x66, 0xd9, 0xef);
pub(crate) const POINT: Color32 = Color32::from_rgb(0xe6, 0xdb, 0x74);
pub(crate) const CONTROL: Color32 = Color32::from_rgb(0xae, 0x81, 0xff);

/// Which kind of point the pointer edits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Target {
    #[default]
    Point,
    Control,
}

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// Scrolls to the cursor's measure on the next frame.
    reveal: bool,
    drag: Option<Drag>,
    draft: Option<Curve>,
    references: references::State,
    prepared: trills::Prepared,
    viewport: viewport::Viewport,
    repeat_drag: crate::view::timeline_repeat::State,
    /// Why the last edit was not applied.
    pub(crate) error: Option<String>,
}

/// A point being dragged; the edit is applied on release.
#[derive(Clone, Copy, Debug)]
struct Drag {
    target: Target,
    /// Index among the points of `target`'s kind.
    index: usize,
    /// From the pointer to the point, so the point does not jump to the pointer.
    offset: egui::Vec2,
    /// The axis when the drag began, kept until release.
    axis: Axis,
    /// Neighbor collision direction and the time the pointer crossed that limit.
    blocked: Option<(i8, f64)>,
}

impl State {
    pub(crate) fn open_at_cursor(&mut self) {
        self.open = true;
        self.reveal = true;
    }
    pub(crate) fn opened() -> Self {
        Self {
            open: true,
            reveal: true,
            ..Self::default()
        }
    }
}

mod axis;
pub(crate) use axis::Axis;

/// (tick, BPM) of each tempo event in the conductor.
pub(crate) fn tempo_events(loaded: &Loaded) -> Vec<(u64, f64)> {
    loaded.music.tracks[0]
        .events
        .iter()
        .filter(|e| e.status == 0xff && e.meta_type == 0x51 && e.data.len() == 3)
        .map(|e| {
            let micros = u32::from_be_bytes([0, e.data[0], e.data[1], e.data[2]]).max(1);
            (e.tick, 60_000_000.0 / f64::from(micros))
        })
        .collect()
}

mod show;
pub use show::show;

/// Width covers all notes and points; height fits the viewport.
fn size(layout: &Layout, curve: Option<&Curve>, height: f32) -> egui::Vec2 {
    let last = curve
        .and_then(|c| c.points.last())
        .map_or(0.0, |p| p.tick as f32 * layout.scale + PX_PER_QUARTER);
    egui::vec2(layout.size.x.max(last), height.max(40.0))
}

/// Screen placement of ticks and BPM.
pub(crate) struct Roll {
    pub(crate) rect: egui::Rect,
    pub(crate) scale: f32,
    pub(crate) axis: Axis,
}

impl Roll {
    pub(crate) fn pos(&self, tick: f64, bpm: f64) -> egui::Pos2 {
        egui::pos2(
            (f64::from(self.rect.left()) + tick * f64::from(self.scale)) as f32,
            self.rect.top() + self.axis.y(bpm, self.rect.height()),
        )
    }

    /// (tick, BPM) at `pos`; the inverse of [`Roll::pos`].
    pub(crate) fn at(&self, pos: egui::Pos2) -> (f64, f64) {
        let axis = self.axis;
        let fraction = f64::from((pos.y - self.rect.top()) / self.rect.height());
        (
            f64::from((pos.x - self.rect.left()) / self.scale),
            axis.high - fraction * (axis.high - axis.low),
        )
    }

    /// Nearest mark of either kind; exact overlaps choose the tempo point.
    pub(crate) fn hit(&self, curve: &Curve, pos: egui::Pos2) -> Option<(Target, usize)> {
        [Target::Point, Target::Control]
            .into_iter()
            .flat_map(|kind| {
                self.marks(curve, kind)
                    .into_iter()
                    .enumerate()
                    .map(move |(index, mark)| (kind, index, self.pinned(mark).distance(pos)))
            })
            .filter(|(_, _, distance)| *distance <= HIT)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(kind, index, _)| (kind, index))
    }

    /// Screen positions of the points of `target`'s kind.
    pub(crate) fn marks(&self, curve: &Curve, target: Target) -> Vec<egui::Pos2> {
        let points = match target {
            Target::Point => &curve.points,
            Target::Control => &curve.controls,
        };
        points
            .iter()
            .map(|p| self.pos(p.tick as f64, p.bpm))
            .collect()
    }
}

pub(crate) mod curve_path;
mod drawing;
mod edges;
mod interaction;
mod note_guides;
mod references;
mod trills;
use super::timeline_repeat as repeat;
pub(crate) mod viewport;
mod window;

mod points;
#[cfg(test)]
pub(crate) mod tests;
