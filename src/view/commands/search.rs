use super::Command;
use regex::{Regex, RegexBuilder};
mod migemo;

/// Keep only the current query; animation/repaint must not rebuild its pattern.
#[derive(Default)]
pub(crate) struct SearchQuery {
    text: String,
    terms: Vec<(String, Option<Regex>)>,
}

impl SearchQuery {
    pub fn matches_text(&self, label: &str) -> bool {
        let lower = label.to_lowercase();
        self.terms.iter().all(|(term, migemo)| {
            lower.contains(term) || migemo.as_ref().is_some_and(|r| r.is_match(label))
        })
    }

    pub fn update(&mut self, query: &str) {
        let text = query.to_lowercase();
        if self.text == text {
            return;
        }
        self.text = text;
        self.terms = self
            .text
            .split_whitespace()
            .map(|term| {
                let pattern = term
                    .bytes()
                    .all(|b| b.is_ascii_alphabetic())
                    .then(|| migemo::pattern(term));
                let regex =
                    pattern.and_then(|p| RegexBuilder::new(&p).case_insensitive(true).build().ok());
                (term.to_owned(), regex)
            })
            .collect();
    }
}

impl Command {
    /// Simple vocabulary belongs to the operation, independently of its visible name.
    pub fn matches_query(&self, query: &SearchQuery) -> bool {
        let label = self.label();
        let label_lower = label.to_lowercase();
        let keys = self.key_label().to_lowercase();
        let words = self.search_words();
        query.terms.iter().all(|(term, migemo)| {
            label_lower.contains(term)
                || keys.contains(term)
                || words.contains(term)
                || migemo.as_ref().is_some_and(|r| r.is_match(&label))
        })
    }

    fn search_words(&self) -> &'static str {
        match self.id.as_str() {
            "help" => "help",
            "quit" => "quit exit",
            "open" => "open file",
            "new" => "new file",
            "add-track" => "add insert track",
            "save" | "save-confirm" => "save",
            "recent" => "recent files history",
            "undo" | "edit-undo" => "undo",
            "stop" | "play-stop" => "stop",
            "play-close" => "close cancel",
            id if id.starts_with("play-") => "play",
            "settings" => "settings",
            "mascot-playback" => "voicevox ずんだもん mascot playback",
            "edit" => "edit",
            "edit-lyrics" => "voicevox 歌詞",
            "mml-open" => "mml insert",
            "mml-measure" => "mml measure overwrite replace",
            "lyrics-measure" => "measure replace",
            "lyrics-confirm" => "voicevox ok confirm",
            "lyrics-cancel" => "voicevox cancel",
            "mixer" => "mixer",
            "layers" => "layers",
            "effects" => "effects",
            "effect-add" => "add effect effects insert",
            "effect-replace" => "replace effect effects",
            "effect-remove" => "remove delete effect effects",
            "effect-bypass" => "bypass effect effects",
            id if id.ends_with("-add") => "add insert",
            "measure-mml-lyrics" => "voicevox mml lyrics measure replace simultaneous",
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
