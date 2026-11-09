//! Pane layout: each host (the main window or a dock window) splits into weighted rows and
//! columns whose leaves each hold one view.
use eframe::egui::{self, Rect};

pub(crate) mod collapse;
pub(crate) mod focus;
pub(crate) mod locate;
pub(crate) mod tabs;
pub(crate) mod window;

#[cfg(test)]
mod tests;

/// A view that can sit in a pane. Each kind appears at most once in a layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ViewKind {
    EventList,
    Score,
    PianoRoll,
    TempoCurve,
    Mixer,
    Analysis,
    Song,
    Scene,
    Keyboard,
    /// The Virtual MIDI keyboard's 88-key piano.
    Piano,
    Help,
    /// The Drum sequencer, Drum kit or Step sequencer, whichever the track takes.
    StepSequencer,
    /// The parameter curve or the Macro variation curve, whichever is open.
    ParameterCurve,
    /// The Track Overview, or its Song version in a song with containers.
    TrackOverview,
}

impl ViewKind {
    /// The name `layout.toml` gives the view.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::EventList => "event_list",
            Self::Score => "score",
            Self::PianoRoll => "piano_roll",
            Self::TempoCurve => "tempo_curve",
            Self::Mixer => "mixer",
            Self::Analysis => "analysis",
            Self::Song => "song",
            Self::Scene => "scene",
            Self::Keyboard => "keyboard",
            Self::Piano => "piano",
            Self::Help => "help",
            Self::StepSequencer => "step_sequencer",
            Self::ParameterCurve => "parameter_curve",
            Self::TrackOverview => "track_overview",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    /// Children left to right, each with its weight.
    Row(Vec<(Node, f32)>),
    /// Children top to bottom, each with its weight.
    Column(Vec<(Node, f32)>),
    View(ViewKind),
    /// Views sharing one pane, one shown at a time.
    Tabs(Vec<ViewKind>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Row,
    Column,
}

/// Where a pane lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Host {
    Root,
    Dock(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dock {
    pub title: String,
    pub placement: crate::store::layout::placement::Placement,
    pub node: Node,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub root: Node,
    pub docks: Vec<Dock>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Error {
    /// A weight is zero, negative or not finite.
    Size(f32),
    /// A row, column or tabs without children.
    Empty,
    Duplicate(ViewKind),
    /// A view that only the main window can hold, placed in a dock window.
    MainOnly(ViewKind),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Size(size) => write!(f, "size must be positive: {size}"),
            Error::Empty => write!(f, "a row, column or tabs needs at least one pane"),
            Error::Duplicate(view) => write!(f, "view placed twice: {view:?}"),
            Error::MainOnly(view) => write!(f, "view stays in the main window: {view:?}"),
        }
    }
}

impl Node {
    /// Leaves in depth-first order, which is also the pane focus order.
    pub(crate) fn views(&self) -> Vec<ViewKind> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<ViewKind>) {
        match self {
            Node::View(view) => out.push(*view),
            Node::Tabs(views) => out.extend(views),
            Node::Row(children) | Node::Column(children) => {
                for (child, _) in children {
                    child.collect(out);
                }
            }
        }
    }

    fn check(&self) -> Result<(), Error> {
        match self {
            Node::View(_) => Ok(()),
            Node::Tabs(views) if views.is_empty() => Err(Error::Empty),
            Node::Tabs(_) => Ok(()),
            Node::Row(children) | Node::Column(children) => {
                if children.is_empty() {
                    return Err(Error::Empty);
                }
                for (child, size) in children {
                    if !size.is_finite() || *size <= 0.0 {
                        return Err(Error::Size(*size));
                    }
                    child.check()?;
                }
                Ok(())
            }
        }
    }
}

impl Layout {
    /// The look without a layout file: the event list, with the score beside it in note mode
    /// unless two tracks already share the list.
    pub(crate) fn default_for(note_mode: bool, split: bool) -> Self {
        let root = if note_mode && !split {
            Node::Row(vec![
                (Node::View(ViewKind::EventList), 1.0),
                (Node::View(ViewKind::Score), 1.0),
            ])
        } else {
            Node::View(ViewKind::EventList)
        };
        Self {
            root,
            docks: Vec::new(),
        }
    }

    fn hosts(&self) -> impl Iterator<Item = (Host, &Node)> {
        std::iter::once((Host::Root, &self.root)).chain(
            self.docks
                .iter()
                .enumerate()
                .map(|(i, dock)| (Host::Dock(i), &dock.node)),
        )
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        let mut seen = Vec::new();
        for (host, node) in self.hosts() {
            node.check()?;
            for view in node.views() {
                if seen.contains(&view) {
                    return Err(Error::Duplicate(view));
                }
                if host != Host::Root && view == ViewKind::EventList {
                    return Err(Error::MainOnly(view));
                }
                seen.push(view);
            }
        }
        Ok(())
    }

    pub(crate) fn host_of(&self, view: ViewKind) -> Option<Host> {
        self.hosts()
            .find(|(_, node)| node.views().contains(&view))
            .map(|(host, _)| host)
    }

    pub(crate) fn contains(&self, view: ViewKind) -> bool {
        self.hosts().any(|(_, node)| node.views().contains(&view))
    }

    /// Every placed view, the main window's first.
    pub(crate) fn views(&self) -> Vec<ViewKind> {
        self.hosts().flat_map(|(_, node)| node.views()).collect()
    }

    /// The tree of `host`.
    pub(crate) fn node(&self, host: Host) -> &Node {
        match host {
            Host::Root => &self.root,
            Host::Dock(index) => &self.docks[index].node,
        }
    }
}

/// The pane after (or before) `current` within one host, wrapping around.
pub(crate) fn focus_step(
    node: &Node,
    current: Option<ViewKind>,
    forward: bool,
) -> Option<ViewKind> {
    let order = node.views();
    let n = order.len();
    let next = match current.and_then(|c| order.iter().position(|v| *v == c)) {
        Some(i) if forward => (i + 1) % n,
        Some(i) => (i + n - 1) % n,
        None if forward => 0,
        None => n.checked_sub(1)?,
    };
    order.get(next).copied()
}

/// Splits `rect` along `axis`, leaving `gap` between neighbours: each `folded` child takes
/// `fixed`, and the open ones share what is left by their `weights`. With every child folded,
/// the space after them stays empty.
pub(crate) fn split_folded(
    rect: Rect,
    axis: Axis,
    weights: &[f32],
    folded: &[bool],
    fixed: f32,
    gap: f32,
) -> Vec<Rect> {
    let is_folded = |i: usize| folded.get(i).copied().unwrap_or(false);
    let folds = (0..weights.len()).filter(|i| is_folded(*i)).count();
    let total: f32 = (0..weights.len())
        .filter(|i| !is_folded(*i))
        .map(|i| weights[i])
        .sum();
    let free = (free_length(rect, axis, weights.len(), gap) - fixed * folds as f32).max(0.0);
    let sizes: Vec<f32> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if is_folded(i) {
                fixed
            } else {
                free * w / total
            }
        })
        .collect();
    place(rect, axis, &sizes, gap)
}

/// The length along `axis` left for `count` children after the gaps between them.
fn free_length(rect: Rect, axis: Axis, count: usize, gap: f32) -> f32 {
    let length = if axis == Axis::Row {
        rect.width()
    } else {
        rect.height()
    };
    (length - gap * count.saturating_sub(1) as f32).max(0.0)
}

/// Lays `sizes` out along `axis` from the start of `rect`, `gap` apart.
fn place(rect: Rect, axis: Axis, sizes: &[f32], gap: f32) -> Vec<Rect> {
    let horizontal = axis == Axis::Row;
    let mut start = 0.0;
    sizes
        .iter()
        .enumerate()
        .map(|(i, size)| {
            let size = *size;
            let from = start + i as f32 * gap;
            start += size;
            if horizontal {
                Rect::from_min_max(
                    egui::pos2(rect.left() + from, rect.top()),
                    egui::pos2(rect.left() + from + size, rect.bottom()),
                )
            } else {
                Rect::from_min_max(
                    egui::pos2(rect.left(), rect.top() + from),
                    egui::pos2(rect.right(), rect.top() + from + size),
                )
            }
        })
        .collect()
}
