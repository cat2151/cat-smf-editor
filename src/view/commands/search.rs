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
        self.matches_search(query, true)
    }

    pub(crate) fn matches_primary_query(&self, query: &SearchQuery) -> bool {
        self.matches_search(query, false)
    }

    fn matches_search(&self, query: &SearchQuery, descriptions: bool) -> bool {
        let label = self.label();
        let label_lower = label.to_lowercase();
        let keys = self.key_label().to_lowercase();
        let description = if descriptions { self.description() } else { "" };
        let description_lower = description.to_lowercase();
        query.terms.iter().all(|(term, migemo)| {
            label_lower.contains(term)
                || keys.contains(term)
                || self.search.contains(term)
                || description_lower.contains(term)
                || migemo.as_ref().is_some_and(|r| {
                    r.is_match(&label) || r.is_match(&self.search) || r.is_match(description)
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::commands::{descriptions::tests::displayed, Screen};

    fn matches(command: &Command, text: &str) -> bool {
        let mut query = SearchQuery::default();
        query.update(text);
        command.matches_query(&query)
    }

    fn bundled_command(screen: Screen, id: &str) -> Command {
        let command = Command::plain(screen, id, &[], |_, _| {});
        displayed(vec![command]).remove(0)
    }

    #[test]
    fn voicevox_and_zundamon_are_mutual_palette_aliases() {
        for (screen, id) in [
            (Screen::Main, "voicevox.singer"),
            (Screen::Settings, "mascot.playback"),
            (Screen::Edit, "edit.lyrics"),
            (Screen::Voicevox, "voicevox.page"),
            (Screen::Singer, "singer.confirm"),
        ] {
            let command = bundled_command(screen, id);
            for text in ["voicevox", "VOICEVOX", "ずんだもん", "zundamon"] {
                assert!(matches(&command, text), "{id}: {text}");
            }
            let both = format!("ずんだもん {}", command.name);
            assert!(matches(&command, &both), "{id}");
        }
        assert!(!matches(
            &bundled_command(Screen::Main, "file.save"),
            "ずんだもん"
        ));
    }

    #[test]
    fn kill_finds_killing_the_voicevox_engine() {
        let command = bundled_command(Screen::Global, "voicevox.kill-quit");
        for text in ["kill", "kill quit", "kill exit", "kill voicevox"] {
            assert!(matches(&command, text), "{text}");
        }
        let named = command.named("VOICEVOX engine を終了してアプリ終了");
        assert!(matches(&named, "shuuryou"));
    }
}
