//! Command identities over the shared query/recent selection history.
use super::{App, Command};
use crate::view::selection_history;
#[derive(Default)]
pub(super) struct History(selection_history::History);
const FILE: &str = "command-palette-history.json";
fn identity(command: &Command) -> String {
    format!("{:?}/{}", command.screen, command.id)
}
impl History {
    pub fn load(app: &mut App) -> Self {
        let fallback = std::mem::take(&mut app.palette.history);
        Self(selection_history::History::load(app, FILE, fallback.0))
    }
    pub fn record(&mut self, query: &str, command: &Command) {
        self.0.record(query, &identity(command));
    }
    pub fn rank(&self, query: &str, command: &Command) -> (u8, usize) {
        self.0.rank(query, &identity(command))
    }
    pub fn save(&self, app: &App) -> Result<(), String> {
        self.0.save(app, FILE)
    }
}
