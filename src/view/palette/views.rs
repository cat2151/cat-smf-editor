//! Browse existing song content without selecting tracks or constructing plugins.
use crate::app::App;
use eframe::egui::{self, Key, Modifiers};

mod catalog;
mod lists;
mod open;
pub(crate) mod preview;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Level {
    Tracks,
    Track(usize),
    Layer(usize, u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Score(usize),
    Mixer,
    DoubleScore,
    Overview,
    Tempo,
    Piano(usize),
    Instruments(usize),
    Effects(usize),
    Macros(usize, u64),
    Variation(usize, u64),
    Parameter(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Level(Level),
    View(View),
}
struct Entry {
    label: String,
    target: Target,
}
impl Entry {
    fn new(label: impl Into<String>, target: Target) -> Self {
        Self {
            label: label.into(),
            target,
        }
    }
}

pub(super) struct State {
    level: Level,
    entries: Vec<Entry>,
    selected: usize,
    parents: Vec<(Level, usize)>,
    help: bool,
    preview: preview::State,
}
impl State {
    pub(super) fn new(app: &App) -> Self {
        let entries = catalog::entries(app, Level::Tracks);
        let selected = entries
            .iter()
            .position(|e| e.target == Target::Level(Level::Track(app.track)))
            .unwrap_or(0);
        Self {
            level: Level::Tracks,
            entries,
            selected,
            parents: vec![],
            help: false,
            preview: Default::default(),
        }
    }
    fn descend(&mut self, app: &App, level: Level) {
        self.parents.push((self.level, self.selected));
        self.level = level;
        self.entries = catalog::entries(app, level);
        self.selected = 0;
    }
    /// False means the root was reached and the palette can be cancelled.
    fn back(&mut self, app: &App) -> bool {
        let Some((level, selected)) = self.parents.pop() else {
            return false;
        };
        self.level = level;
        self.entries = catalog::entries(app, level);
        self.selected = selected.min(self.entries.len().saturating_sub(1));
        true
    }
}

/// Enter opens a leaf; horizontal movement only changes the browsed hierarchy.
pub(super) fn show(app: &mut App, ctx: &egui::Context, state: &mut State, fresh: bool) -> bool {
    let mut activate = false;
    let mut descend = false;
    let mut back = false;
    if fresh {
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            {
                if !super::commands::matches(modifiers, Modifiers::NONE) {
                    continue;
                }
                if key == Key::F1 && !repeat {
                    state.help = !state.help;
                    continue;
                }
                if state.help {
                    if key == Key::Escape && !repeat {
                        state.help = false;
                    }
                    continue;
                }
                match key {
                    Key::J | Key::ArrowDown => {
                        state.selected =
                            (state.selected + 1).min(state.entries.len().saturating_sub(1));
                    }
                    Key::K | Key::ArrowUp => {
                        state.selected = state.selected.saturating_sub(1);
                    }
                    Key::H | Key::ArrowLeft | Key::Escape if !repeat => back = true,
                    Key::L | Key::ArrowRight if !repeat => descend = true,
                    Key::Enter if !repeat => activate = true,
                    _ => {}
                }
            }
        }
    }
    // No text input in this mode: hjkl select candidates rather than edit a query.
    if back {
        if !state.back(app) {
            return true;
        }
    } else if let Some(target) = state.entries.get(state.selected).map(|e| e.target) {
        match target {
            Target::Level(level) if activate || descend => state.descend(app, level),
            Target::View(view) if activate => {
                open::show(app, view);
                ctx.request_repaint();
                return true;
            }
            _ => {}
        }
    }
    let target = lists::show(app, ctx, state, fresh);
    if let Some(Target::View(view)) = target {
        preview::show(app, ctx, &mut state.preview, view);
    }
    false
}
