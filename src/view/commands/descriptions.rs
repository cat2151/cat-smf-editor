//! Editable wording is loaded once per palette opening, never during rendering/search.
use super::{Command, Screen};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
#[cfg(test)]
mod id_tests;
#[cfg(test)]
pub(crate) mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definitions {
    #[serde(default)]
    screens: HashMap<Screen, toml::Spanned<String>>,
    command: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    ids: toml::Spanned<Vec<String>>,
    screen: Option<Screen>,
    name: Option<String>,
    tooltip: Option<String>,
    search: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct Text {
    name: Option<String>,
    tooltip: Option<String>,
    search: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
struct Entries {
    commands: HashMap<(String, Option<Screen>), Text>,
    screens: HashMap<Screen, String>,
}

fn line_at(text: &str, offset: usize) -> usize {
    text[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}

fn parse(text: &str) -> Result<Entries, String> {
    let text = text.trim_start_matches('\u{feff}');
    let definitions: Definitions = toml::from_str(text).map_err(|e| {
        let line = e.span().map_or(1, |span| line_at(text, span.start));
        format!("{line}: {}", e.message())
    })?;
    let mut entries = Entries::default();
    for (screen, name) in definitions.screens {
        if name.get_ref().is_empty() {
            let line = line_at(text, name.span().start);
            return Err(format!("{line}: 画面名は空にできません"));
        }
        entries.screens.insert(screen, name.into_inner());
    }
    for item in definitions.command {
        let line = line_at(text, item.ids.span().start);
        let fields = [&item.name, &item.tooltip, &item.search];
        if fields.iter().all(|field| field.is_none()) {
            return Err(format!("{line}: name・tooltip・searchのどれかが必要です"));
        }
        if fields
            .iter()
            .any(|field| field.as_ref().is_some_and(|text| text.is_empty()))
        {
            return Err(format!("{line}: name・tooltip・searchは空にできません"));
        }
        let ids = item.ids.into_inner();
        if ids.is_empty() || ids.iter().any(String::is_empty) {
            return Err(format!("{line}: command IDがありません"));
        }
        let entry = Text {
            name: item.name,
            tooltip: item.tooltip,
            search: item.search,
        };
        for id in ids {
            if entries
                .commands
                .insert((id.clone(), item.screen), entry.clone())
                .is_some()
            {
                return Err(format!("{line}: command IDが重複しています: {id}"));
            }
        }
    }
    Ok(entries)
}

#[derive(Default)]
pub(crate) struct Catalog {
    path: Option<PathBuf>,
    entries: Arc<Entries>,
    error: Option<String>,
}

impl Catalog {
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            ..Self::default()
        }
    }

    /// Called only when opening the palette: App/State defaults must do no IO.
    pub(crate) fn reload(&mut self, error: &mut Option<String>) {
        let path = self.path.clone().or_else(external_path);
        self.load(path, error);
    }

    /// Replace the snapshot only after the complete file has passed validation.
    fn load(&mut self, path: Option<PathBuf>, error: &mut Option<String>) {
        if self.entries.commands.is_empty() {
            self.entries = bundled();
        }
        let result = match &path {
            Some(path) => {
                #[cfg(test)]
                tests::record(tests::Work::Read);
                match std::fs::read_to_string(path) {
                    Ok(text) => parse(&text).map(Arc::new),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(bundled()),
                    Err(e) => Err(e.to_string()),
                }
            }
            None => Ok(bundled()),
        };
        match result {
            Ok(entries) => {
                self.entries = entries;
                if self.error.is_some() && *error == self.error {
                    *error = None;
                }
                self.error = None;
            }
            Err(reason) => {
                let message = format!("{}:{reason}", path.as_ref().unwrap().display());
                *error = Some(message.clone());
                self.error = Some(message);
            }
        }
    }

    /// A screen's entry wins per field, so it can restate only what differs.
    fn text(&self, command: &Command) -> (Option<&str>, Option<&str>, Option<&str>) {
        let id = command.text_id.unwrap_or(&command.id);
        let entries = &self.entries.commands;
        let scoped = entries.get(&(id.to_owned(), Some(command.screen)));
        let shared = entries.get(&(id.to_owned(), None));
        let pick = |field: fn(&Text) -> Option<&str>| {
            scoped.and_then(field).or_else(|| shared.and_then(field))
        };
        (
            pick(|text| text.name.as_deref()),
            pick(|text| text.tooltip.as_deref()),
            pick(|text| text.search.as_deref()),
        )
    }

    pub(crate) fn apply(&self, mut commands: Vec<Command>) -> Vec<Command> {
        for command in &mut commands {
            let (name, tooltip, search) = self.text(command);
            let name = name.unwrap_or(&command.id).to_owned();
            command.description = tooltip.unwrap_or_default().to_owned();
            command.search = search.unwrap_or_default().to_lowercase();
            command.screen_name = match self.entries.screens.get(&command.screen) {
                Some(name) => name.clone(),
                None => format!("{:?}", command.screen),
            };
            if command.name.is_empty() {
                command.name = name;
            }
        }
        commands
    }
}

fn bundled() -> Arc<Entries> {
    #[cfg(test)]
    tests::record(tests::Work::Bundle);
    static ENTRIES: OnceLock<Arc<Entries>> = OnceLock::new();
    Arc::clone(ENTRIES.get_or_init(|| {
        Arc::new(
            parse(include_str!("../../../config/command-palette.toml"))
                .expect("bundled command palette definitions must be valid"),
        )
    }))
}

/// Only a development build has the editable source; other exes use the bundle.
fn source_path(executable: &Path, project: &Path) -> Option<PathBuf> {
    executable
        .starts_with(project.join("target"))
        .then(|| project.join("config/command-palette.toml"))
}

pub(crate) fn external_path() -> Option<PathBuf> {
    #[cfg(test)]
    tests::record(tests::Work::Locate);
    // Cache only the location; external file contents are reread on every opening.
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let executable = std::env::current_exe().ok()?;
        let executable = executable.canonicalize().unwrap_or(executable);
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project = project.canonicalize().unwrap_or(project);
        source_path(&executable, &project)
    })
    .clone()
}

impl Command {
    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    /// Rows generated per key or item share one definition.
    pub(crate) fn text_id(mut self, id: &'static str) -> Self {
        self.text_id = Some(id);
        self
    }
}
