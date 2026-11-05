//! Names in the matrix, where "Pattern" would repeat in every cell.
use super::*;
use cat_smf_editor::score::pattern::Kind;
use std::collections::BTreeMap;

const SHORT_NAME_LENGTH: usize = 8;

/// The first letter, then the rest without vowels, cut to eight characters.
fn short_name(name: &str) -> String {
    let mut chars = name.chars();
    let first = chars.next();
    first
        .into_iter()
        .chain(chars.filter(|c| !"aeiouAEIOU".contains(*c)))
        .take(SHORT_NAME_LENGTH)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// Candidates in their list order, a pattern named by its container and its place there.
/// A container's Loop stays "Loop" and takes no place.
pub(super) fn cells(a: &App, p: Pane) -> Vec<(PatternOrScene, String)> {
    let Some(l) = a.loaded.as_ref() else {
        return vec![];
    };
    let Some((catalog, _)) = &l.arrangement else {
        return vec![];
    };
    let mut serials = BTreeMap::<usize, usize>::new();
    choices(a, p)
        .into_iter()
        .map(|(part, label)| match part {
            PatternOrScene::Pattern(id) => {
                let Some(def) = catalog.patterns.get(&id) else {
                    return (part, label);
                };
                if def.kind == Kind::Loop {
                    return (part, def.kind.label().to_owned());
                }
                let serial = serials.entry(def.track).or_default();
                *serial += 1;
                let name = cat_smf_editor::score::pattern::container_name(
                    &l.music,
                    def.container,
                    def.track,
                );
                (part, format!("{} {serial}", short_name(&name)))
            }
            PatternOrScene::Scene(_) => (part, label),
        })
        .collect()
}

/// The candidates the query finds, by pattern name or by the name shown on their card.
pub(in crate::view::patterns::workspace) fn found(
    a: &App,
    p: Pane,
) -> Vec<(PatternOrScene, String)> {
    let search = &a.patterns.workspace.selection(p).search;
    choices(a, p)
        .into_iter()
        .zip(cells(a, p))
        .filter(|((_, name), (_, shown))| search.matches_text(name) || search.matches_text(shown))
        .map(|(_, shown)| shown)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::short_name;

    #[test]
    fn short_names_keep_the_first_letter_and_drop_later_vowels() {
        assert_eq!(short_name("Chord"), "Chrd");
        assert_eq!(short_name("Organ"), "Orgn");
        assert_eq!(short_name("Acoustic Bass"), "Acstc Bs");
        assert_eq!(short_name("Electric Piano"), "Elctrc P");
        assert_eq!(short_name("ドラム"), "ドラム");
    }

    #[test]
    fn a_cut_at_a_space_leaves_no_trailing_space() {
        assert_eq!(short_name("Strngss Pad"), "Strngss");
    }
}
