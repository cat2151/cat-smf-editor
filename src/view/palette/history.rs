//! Stable command identities, most recently selected first, separately from document state.
use super::{App, Command};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub(super) struct History {
    queries: BTreeMap<String, Vec<String>>,
    recent: Vec<String>,
    #[serde(skip)]
    unreadable: bool,
}
fn path(app: &App) -> Option<PathBuf> {
    Some(
        app.config
            .path
            .as_ref()?
            .with_file_name("command-palette-history.json"),
    )
}
fn identity(command: &Command) -> String {
    format!("{:?}/{}", command.screen, command.id)
}
impl History {
    pub fn load(app: &mut App) -> Self {
        let Some(path) = path(app) else {
            return std::mem::take(&mut app.palette.history);
        };
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(history) => history,
                Err(error) => {
                    app.config.error = Some(format!("Cannot read {}: {error}", path.display()));
                    Self {
                        unreadable: true,
                        ..Self::default()
                    }
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                app.config.error = Some(format!("Cannot read {}: {error}", path.display()));
                Self {
                    unreadable: true,
                    ..Self::default()
                }
            }
        }
    }
    pub fn record(&mut self, query: &str, command: &Command) {
        let id = identity(command);
        for list in [
            self.queries.entry(query.to_owned()).or_default(),
            &mut self.recent,
        ] {
            list.retain(|previous| previous != &id);
            list.insert(0, id.clone());
        }
    }
    pub fn rank(&self, query: &str, command: &Command) -> (u8, usize) {
        let id = identity(command);
        if let Some(position) = self
            .queries
            .get(query)
            .and_then(|list| list.iter().position(|s| s == &id))
        {
            return (0, position);
        }
        self.recent
            .iter()
            .position(|s| s == &id)
            .map_or((2, 0), |p| (1, p))
    }
    pub fn save(&self, app: &App) -> Result<(), String> {
        if self.unreadable {
            return Ok(());
        }
        let Some(path) = path(app) else { return Ok(()) };
        let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let write = || -> Result<(), Box<dyn std::error::Error>> {
            use std::io::Write;
            std::fs::create_dir_all(path.parent().unwrap())?;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temporary, &path)?;
            Ok(())
        };
        write().map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            format!("Cannot save {}: {e}", path.display())
        })
    }
}
