use super::Command;
use regex::{Regex, RegexBuilder};
mod migemo;

/// Keep only the current query; animation/repaint must not rebuild its pattern.
#[derive(Default)]
pub(crate) struct SearchQuery {
    text: String,
    migemo: Option<Regex>,
}

impl SearchQuery {
    pub fn update(&mut self, query: &str) {
        let text = query.to_lowercase();
        if self.text == text {
            return;
        }
        self.text = text;
        self.migemo = None;
        // Native text, punctuation and key chords keep literal substring semantics.
        if self.text.is_empty() || !self.text.bytes().all(|b| b.is_ascii_alphabetic()) {
            return;
        }
        let pattern = migemo::pattern(&self.text);
        self.migemo = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
            .ok();
    }
}

impl Command {
    /// Simple vocabulary belongs to the operation, independently of its visible name.
    pub fn matches_query(&self, query: &SearchQuery) -> bool {
        let label = self.label();
        label.to_lowercase().contains(&query.text)
            || self.key_label().to_lowercase().contains(&query.text)
            || self.english_search().contains(&query.text)
            || query.migemo.as_ref().is_some_and(|r| r.is_match(&label))
    }

    fn english_search(&self) -> &'static str {
        match self.id.as_str() {
            "help" => "help",
            "quit" => "quit exit",
            "open" => "open file",
            "new" => "new file",
            "save" | "save-confirm" => "save",
            "recent" => "recent files history",
            "undo" | "edit-undo" => "undo",
            "stop" | "play-stop" => "stop",
            "play-close" => "close cancel",
            id if id.starts_with("play-") => "play",
            "settings" => "settings",
            "edit" => "edit",
            "mixer" => "mixer",
            "layers" => "layers",
            "effects" => "effects",
            "patch" => "sound instrument",
            "volume-down" => "volume down",
            "volume-up" => "volume up",
            id if id.ends_with("-cancel") => "cancel",
            id if id.ends_with("-confirm") => "ok confirm",
            id if id.ends_with("-close") => "close",
            _ => "",
        }
    }
}
