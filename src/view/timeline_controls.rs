//! Horizontal timeline contract. Adapters supply content coordinates, never a second
//! scroll offset for the overview, repeat row or zoom anchor.
use super::{timeline_minimap, timeline_repeat, timeline_zoom};
use eframe::egui;

mod horizontal;
mod viewport;
pub(crate) use horizontal::Horizontal;
mod zoom;
pub(crate) use viewport::Viewport;
pub(crate) use zoom::Zoom;
#[cfg(test)]
mod tests;

macro_rules! screens {
    ($($name:ident),+ $(,)?) => {
        #[derive(Clone,Copy,Debug,PartialEq,Eq,Hash)]
        pub(crate) enum Screen { $($name),+ }
        impl Screen {
            pub const ALL: [Self; [$(stringify!($name)),+].len()] = [$(Self::$name),+];
        }
    };
}

// This single declaration is the production registry and the UI fixture census.
screens!(
    Staff,
    Song,
    PianoRoll,
    Tempo,
    TrackOverview,
    Parameter,
    Scene
);

impl Screen {
    pub fn policy(self) -> Click {
        match self {
            Self::Song => Click::Song,
            Self::Scene => Click::Scene,
            Self::Parameter => Click::Cursor,
            Self::Staff | Self::PianoRoll | Self::Tempo | Self::TrackOverview => Click::SeekPlaying,
        }
    }

    pub fn has_bars(self) -> bool {
        self != Self::Scene
    }
}

/// Only the stopped-click side effect differs. Drag never produces a click tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Click {
    Song,
    Scene,
    Cursor,
    SeekPlaying,
}

/// One frame's actual time-bearing content connection. `scroll` is measured from
/// its content origin; `viewport` excludes fixed labels, insets and scrollbars.
pub(crate) struct Binding<'a> {
    pub screen: Screen,
    pub axis: &'a dyn timeline_repeat::TickAxis,
    pub viewport: egui::Rect,
    pub scroll: f32,
    pub end: u64,
    /// A held zoom-bar range drag, in px from the viewport's left edge.
    pub zoom_range: Option<(f32, f32)>,
}

impl Binding<'_> {
    pub fn width(&self) -> f32 {
        self.viewport.width().max(0.0)
    }

    pub fn clamp(&self, scroll: f32) -> f32 {
        scroll.clamp(0.0, (self.axis.x(self.end) - self.width()).max(0.0))
    }

    pub fn visible(&self) -> (f64, f64) {
        (
            self.axis.tick_at(self.scroll).clamp(0.0, self.end as f64),
            self.axis
                .tick_at(self.scroll + self.width())
                .clamp(0.0, self.end as f64),
        )
    }

    pub fn center(&self, tick: f64) -> f32 {
        self.clamp(self.axis.x_at(tick) - self.width() / 2.0)
    }

    /// Tick range visible after the held zoom range is released.
    pub fn forecast(&self) -> Option<(f64, f64)> {
        let (a, b) = self.zoom_range?;
        let width = (a - b).abs() / timeline_zoom::FIT;
        let left = (self.scroll + (a + b) / 2.0 - width / 2.0)
            .clamp(0.0, (self.axis.x(self.end) - width).max(0.0));
        let tick = |x: f32| self.axis.tick_at(x).clamp(0.0, self.end as f64);
        Some((tick(left), tick(left + width)))
    }
}

/// Recalculation shared by mouse, keyboard, fit-range and wheel zoom.
pub(crate) fn zoom(scroll: f32, width: f32, from: f32, to: f32, anchor: f32) -> (f32, f32) {
    let to = timeline_zoom::clamp(to);
    (
        to,
        timeline_zoom::rescroll(scroll, anchor.clamp(0.0, width), from, to, width),
    )
}

/// Finish a ScrollArea frame before drawing the next overview or accepting zoom.
pub(crate) fn finish(viewport: egui::Rect, offset: f32) -> (egui::Rect, f32) {
    (viewport, offset.max(0.0))
}

pub(crate) use timeline_minimap::State as Minimap;

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Snapshot {
    pub scroll: f32,
    pub width: f32,
    pub end_x: f32,
}
