//! Keeps `config/command-palette.toml` and the command ids in the source in step.
use super::*;
use crate::view::commands;
use regex::Regex;
use std::collections::BTreeSet;

/// A file dialog filter in a command-building file, not a command id.
const NOT_ID: &str = "run.exe";
/// Families whose rows get their names from runtime data, so only the tooltip is defined.
const NAMED_IN_CODE: [&str; 2] = ["recent.entry", "track.entry"];

#[derive(Default)]
struct Sources {
    /// String literals in files that build commands.
    literals: BTreeSet<String>,
    /// Every string literal in production code, wherever it stands.
    anywhere: BTreeSet<String>,
    /// `format!` ids such as `macro.count-{digit}`, as patterns.
    families: Vec<Regex>,
}

/// Inline test modules sit at the end of a file.
fn production(text: &str) -> &str {
    let inline = Regex::new(r"#\[cfg\(test\)\]\s+mod tests \{").unwrap();
    inline.find(text).map_or(text, |m| &text[..m.start()])
}

fn scan(text: &str, sources: &mut Sources) {
    let text = production(text);
    let literal = Regex::new(r#""([a-z][a-z0-9-]*\.[a-z0-9-]+)""#).unwrap();
    let builds = Regex::new(r"(^|[^:\w])(Command::(plain|new|operation)|Spec::plain)\b")
        .unwrap()
        .is_match(text);
    for found in literal.captures_iter(text) {
        sources.anywhere.insert(found[1].to_owned());
        if builds {
            sources.literals.insert(found[1].to_owned());
        }
    }
    let family = Regex::new(r#"format!\(\s*"([a-z-]+\.[a-z-]+-)\{[^"]*\}""#).unwrap();
    for found in family.captures_iter(text) {
        let pattern = format!("^{}.+$", regex::escape(&found[1]));
        sources.families.push(Regex::new(&pattern).unwrap());
    }
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs")
            && !path.to_string_lossy().contains("test")
        {
            out.push(path);
        }
    }
}

fn sources() -> Sources {
    let mut paths = Vec::new();
    files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/view"),
        &mut paths,
    );
    let mut sources = Sources::default();
    for path in paths {
        scan(&std::fs::read_to_string(path).unwrap(), &mut sources);
    }
    sources
}

/// Ids built from a key or a digit, each of which has its own definition.
fn generated() -> Vec<String> {
    let mut ids: Vec<_> = (0..10)
        .map(|digit| format!("macro.count-{digit}"))
        .collect();
    for (key, ..) in crate::view::list_navigation::bindings() {
        if !matches!(
            key,
            eframe::egui::Key::ArrowUp | eframe::egui::Key::ArrowDown
        ) {
            ids.push(format!("parameter.nav-{key:?}"));
            ids.push(format!("singer.nav-{key:?}"));
        }
    }
    ids
}

/// Ids the source uses without a definition, and defined ids the source never uses.
fn mismatches(sources: &Sources, entries: &Entries) -> (Vec<String>, Vec<String>) {
    let defined: BTreeSet<_> = entries.commands.keys().map(|(id, _)| id.clone()).collect();
    let undefined = sources
        .literals
        .iter()
        .filter(|id| *id != NOT_ID)
        .cloned()
        .chain(generated())
        .filter(|id| !defined.contains(id))
        .collect();
    let unused = defined
        .into_iter()
        .filter(|id| {
            !sources.anywhere.contains(id) && !sources.families.iter().any(|f| f.is_match(id))
        })
        .collect();
    (undefined, unused)
}

#[test]
fn every_defined_id_is_in_the_source_and_every_fixed_id_is_defined() {
    let sources = sources();
    let (undefined, unused) = mismatches(&sources, &bundled());
    assert_eq!(undefined, Vec::<String>::new());
    assert_eq!(unused, Vec::<String>::new());
}

#[test]
fn every_defined_id_reads_as_target_dot_operation() {
    let shape = Regex::new(r"^[a-z][a-z-]*\.[a-z0-9][a-zA-Z0-9-]*$").unwrap();
    let odd: Vec<_> = bundled()
        .commands
        .keys()
        .map(|(id, _)| id.clone())
        .filter(|id| !shape.is_match(id))
        .collect();
    assert_eq!(odd, Vec::<String>::new());
}

#[test]
fn a_mistyped_id_is_reported_from_both_sides() {
    let text = include_str!("../../../../config/command-palette.toml");
    let typo = text.replace("ids = [\"file.save\"]", "ids = [\"file.sav\"]");
    assert_ne!(typo, text);
    let (undefined, unused) = mismatches(&sources(), &parse(&typo).unwrap());
    assert_eq!(undefined, ["file.save"]);
    assert_eq!(unused, ["file.sav"]);
}

#[test]
fn every_defined_id_has_a_name_unless_runtime_data_names_it() {
    let entries = bundled();
    let named: BTreeSet<_> = entries
        .commands
        .iter()
        .filter(|(_, text)| text.name.is_some())
        .map(|((id, _), _)| id.as_str())
        .collect();
    let nameless: BTreeSet<_> = entries
        .commands
        .keys()
        .map(|(id, _)| id.as_str())
        .filter(|id| !named.contains(id))
        .collect();
    assert_eq!(nameless, BTreeSet::from(NAMED_IN_CODE));
}

#[test]
fn every_row_of_every_screen_shows_a_defined_name() {
    let app = crate::view::palette::tests::app();
    for screen in bundled().screens.keys() {
        for command in tests::displayed(commands::definitions(&app, *screen)) {
            assert_ne!(command.name, command.id, "{screen:?}");
        }
    }
}
