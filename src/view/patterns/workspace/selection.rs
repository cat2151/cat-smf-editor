use super::{Pane, PatternOrScene};

pub struct Selection {
    pub row: usize,
    pub measure: u64,
    pub choice: Option<PatternOrScene>,
    pub query: String,
    pub search: crate::view::commands::SearchQuery,
    pub focus: bool,
    pub owned: bool,
    pub composing: bool,
    /// Song enters from the library; cell position and chosen part survive a return.
    pub placing: bool,
    pub reveal_choice: bool,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            row: 0,
            measure: 1,
            choice: None,
            query: String::new(),
            search: Default::default(),
            focus: false,
            owned: false,
            composing: false,
            placing: false,
            reveal_choice: true,
        }
    }
}
impl Selection {
    pub fn cells(&self, pane: Pane) -> bool {
        pane == Pane::Scene || self.placing
    }
}
