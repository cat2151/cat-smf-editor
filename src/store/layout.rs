//! `layout.toml`: which view sits in which pane. Re-read whenever the file changes; the app never
//! writes back to it.
use crate::view::dock::{Dock, Layout, Node, ViewKind};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

#[cfg(test)]
const TEMPLATE: &str = include_str!("../../config/layout.toml");
pub(crate) mod location;
pub(crate) mod monitors;
pub(crate) mod placement;
mod runtime;
use placement::{MonitorPlacement, Placement, Resolved};
pub(crate) use runtime::update;
pub(crate) const POLL: Duration = Duration::from_millis(500);

/// The panes as written in the file. Without a root tree the main window keeps the built-in
/// look, which depends on the edit mode.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Spec {
    pub root: Option<Node>,
    pub docks: Vec<Dock>,
}

impl Spec {
    pub(crate) fn resolve(&self, note_mode: bool, split: bool) -> Layout {
        let mut layout = Layout::default_for(note_mode, split);
        if let Some(root) = &self.root {
            layout.root = root.clone();
        }
        layout.docks = self.docks.clone();
        layout
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    root: Option<Tree>,
    #[serde(default)]
    window: Vec<Window>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Tree {
    view: Option<ViewKind>,
    row: Option<Vec<Child>>,
    column: Option<Vec<Child>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Child {
    view: Option<ViewKind>,
    row: Option<Vec<Child>>,
    column: Option<Vec<Child>>,
    /// Share of a `row`.
    width: Option<f32>,
    /// Share of a `column`.
    height: Option<f32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    title: Option<String>,
    rect: Option<[f32; 4]>,
    monitor_placement: Option<MonitorPlacement>,
    view: Option<ViewKind>,
    row: Option<Vec<Child>>,
    column: Option<Vec<Child>>,
}

fn node(
    view: Option<ViewKind>,
    row: Option<Vec<Child>>,
    column: Option<Vec<Child>>,
) -> Result<Option<Node>, String> {
    let children = |list: Vec<Child>, in_row: bool| -> Result<Vec<(Node, f32)>, String> {
        list.into_iter()
            .map(|child| {
                let (share, other) = if in_row {
                    (child.width, child.height)
                } else {
                    (child.height, child.width)
                };
                if other.is_some() {
                    return Err(if in_row {
                        "a pane in a `row` takes `width`, not `height`".to_owned()
                    } else {
                        "a pane in a `column` takes `height`, not `width`".to_owned()
                    });
                }
                let size = share.unwrap_or(1.0);
                node(child.view, child.row, child.column)?
                    .map(|node| (node, size))
                    .ok_or_else(|| "a pane needs one of `view`, `row` or `column`".to_owned())
            })
            .collect()
    };
    match (view, row, column) {
        (None, None, None) => Ok(None),
        (Some(view), None, None) => Ok(Some(Node::View(view))),
        (None, Some(row), None) => Ok(Some(Node::Row(children(row, true)?))),
        (None, None, Some(column)) => Ok(Some(Node::Column(children(column, false)?))),
        _ => Err("use only one of `view`, `row` and `column` in a pane".to_owned()),
    }
}

pub(crate) fn parse(text: &str) -> Result<Spec, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let root = file.root.unwrap_or_default();
    let root = node(root.view, root.row, root.column)?;
    let docks = file
        .window
        .into_iter()
        .map(|window| {
            let node = node(window.view, window.row, window.column)?
                .ok_or_else(|| "window has no panes".to_owned())?;
            let title = window.title.unwrap_or_else(|| window_title(&node));
            let placement = match (window.rect, window.monitor_placement) {
                (Some(rect), None) => {
                    let [x, y, width, height] = rect;
                    if ![x, y, width, height].iter().all(|n| n.is_finite())
                        || width <= 0.0
                        || height <= 0.0
                    {
                        return Err(format!(
                            "window `{title}`: rect requires finite x/y and positive width/height"
                        ));
                    }
                    Placement::Desktop(rect)
                }
                (None, Some(placement)) => {
                    placement
                        .validate()
                        .map_err(|e| format!("window `{title}`: {e}"))?;
                    Placement::Monitor(placement)
                }
                _ => {
                    return Err(format!(
                        "window `{title}` needs exactly one of rect or monitor_placement"
                    ))
                }
            };
            Ok(Dock {
                title,
                placement,
                node,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let spec = Spec { root, docks };
    let mut titles = std::collections::HashSet::new();
    for dock in &spec.docks {
        if dock.title == "cat-smf-editor" || !titles.insert(&dock.title) {
            return Err(format!("duplicate window title: {}", dock.title));
        }
    }
    // The built-in root holds every view it can show, so a file without a root tree must not
    // place those views in a dock window either.
    spec.resolve(true, false)
        .validate()
        .map_err(|e| e.to_string())?;
    Ok(spec)
}

/// The layout in use, the file it came from and the last read failure.
#[derive(Default)]
pub(crate) struct Watch {
    path: Option<PathBuf>,
    modified: Option<SystemTime>,
    checked: Option<Instant>,
    pub spec: Spec,
    pub error: Option<String>,
    pub resolved: Vec<Option<Resolved>>,
    candidate: Option<Spec>,
}

impl Watch {
    /// Read only: a missing file means the built-in layout, never an auto-generated copy.
    #[cfg(test)]
    pub(crate) fn open(directory: &Path) -> Self {
        Self::at(directory.join("layout.toml"))
    }

    pub(crate) fn at(path: PathBuf) -> Self {
        let mut watch = Self {
            path: Some(path.clone()),
            ..Self::default()
        };
        watch.reload();
        watch
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn watching(&self) -> bool {
        self.path.is_some()
    }

    /// Re-reads the file when its modification time changed, at most once per `POLL`.
    pub(crate) fn poll(&mut self, now: Instant) {
        if self.path.is_none() || self.checked.is_some_and(|at| now - at < POLL) {
            return;
        }
        self.checked = Some(now);
        self.reload();
        if self.candidate.as_ref().is_some_and(|s| {
            s.docks
                .iter()
                .any(|d| matches!(d.placement, Placement::Monitor(_)))
        }) {
            self.resolve_candidate();
        }
    }

    fn reload(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let modified = match std::fs::metadata(path).and_then(|m| m.modified()) {
            Ok(modified) => Some(modified),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                self.candidate = None;
                self.modified = None;
                self.error = Some(format!("{}: {error}", path.display()));
                return;
            }
        };
        if modified == self.modified {
            return;
        }
        self.modified = modified;
        if modified.is_none() {
            self.spec = Spec::default();
            self.resolved.clear();
            self.candidate = None;
            self.error = None;
            return;
        }
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse(&text))
        {
            Ok(spec) => {
                self.candidate = Some(spec);
                self.resolve_candidate();
            }
            Err(error) => {
                self.candidate = None;
                self.error = Some(format!("{}: {error}", path.display()));
            }
        }
    }

    fn resolve_candidate(&mut self) {
        let Some(spec) = &self.candidate else {
            return;
        };
        match resolve(spec) {
            Ok(resolved) => {
                self.spec = spec.clone();
                self.resolved = resolved;
                self.error = None;
            }
            Err(error) => {
                self.error = Some(format!(
                    "{}: {error}",
                    self.path.as_ref().unwrap().display()
                ))
            }
        }
    }
}

fn resolve(spec: &Spec) -> Result<Vec<Option<Resolved>>, String> {
    let screens = if spec
        .docks
        .iter()
        .any(|d| matches!(d.placement, Placement::Monitor(_)))
    {
        monitors::enumerate()?
    } else {
        Vec::new()
    };
    spec.docks
        .iter()
        .map(|dock| match &dock.placement {
            Placement::Desktop(_) => Ok(None),
            Placement::Monitor(placement) => placement
                .resolve(&screens)
                .map(Some)
                .map_err(|e| format!("window `{}`: {e}", dock.title)),
        })
        .collect()
}

fn window_title(node: &Node) -> String {
    let mut title = "cat-smf-editor".to_owned();
    for view in node.views() {
        title.push_str(" · ");
        title.push_str(match view {
            ViewKind::EventList => "event_list",
            ViewKind::Score => "score",
            ViewKind::PianoRoll => "piano_roll",
            ViewKind::TempoCurve => "tempo_curve",
            ViewKind::Mixer => "mixer",
            ViewKind::Analysis => "analysis",
            ViewKind::Song => "song",
            ViewKind::Scene => "scene",
            ViewKind::Keyboard => "keyboard",
            ViewKind::Piano => "piano",
            ViewKind::Help => "help",
            ViewKind::StepSequencer => "step_sequencer",
            ViewKind::ParameterCurve => "parameter_curve",
            ViewKind::TrackOverview => "track_overview",
        });
    }
    title
}

#[cfg(test)]
pub(crate) mod tests;
