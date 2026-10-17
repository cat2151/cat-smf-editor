//! This machine's patch roots: discovered by the player, kept in the regenerable cache (ADR 0008).
// Tests answer from `Assumed` instead of starting the player.
#![cfg_attr(test, allow(dead_code))]
use cat_smf_editor::{
    host::PluginKey,
    patch_roots::{Plugin, Roots, Table},
};
use std::{io::Write, path::PathBuf};

pub struct Cache {
    file: Option<PathBuf>,
    table: Option<Table>,
}

impl Cache {
    pub fn new(application_data: Option<PathBuf>) -> Self {
        Self {
            file: application_data
                .map(|p| cat_smf_editor::storage_paths::cache(&p).join("patch-roots-v1.json")),
            table: None,
        }
    }

    /// The player is asked at most once per lookup, however many references miss.
    pub fn lookup(&mut self, player: Option<PathBuf>) -> Lookup<'_> {
        Lookup {
            cache: self,
            player,
            refreshed: false,
        }
    }

    /// A missing or unreadable file is a miss that the next refresh replaces.
    fn table(&mut self) -> &Table {
        let file = &self.file;
        self.table.get_or_insert_with(|| {
            file.as_ref()
                .and_then(|file| std::fs::read(file).ok())
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or_default()
        })
    }

    fn refresh(&mut self, player: Option<PathBuf>) -> Result<(), String> {
        let output = super::state_request::headless(player, "--patch-roots")?
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Player root discovery failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let table: Table = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        if let Some(file) = &self.file {
            save(file, &output.stdout).map_err(|e| format!("{}: {e}", file.display()))?;
        }
        self.table = Some(table);
        Ok(())
    }
}

fn save(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path)
}

pub struct Lookup<'a> {
    cache: &'a mut Cache,
    player: Option<PathBuf>,
    refreshed: bool,
}

impl Roots for Lookup<'_> {
    #[cfg(not(test))]
    fn plugin(&mut self, key: &PluginKey, refresh: bool) -> Result<Plugin, String> {
        if refresh && !self.refreshed {
            self.refreshed = true;
            self.cache.refresh(self.player.clone())?;
        }
        self.cache.table().plugin(key)
    }

    #[cfg(test)]
    fn plugin(&mut self, key: &PluginKey, refresh: bool) -> Result<Plugin, String> {
        Assumed.plugin(key, refresh)
    }

    #[cfg(test)]
    fn exists(&self, path: &std::path::Path) -> bool {
        Assumed.exists(path)
    }
}

#[cfg(test)]
thread_local! {
    static BUNDLES: std::cell::RefCell<std::collections::HashMap<String, String>> =
        Default::default();
    static ROOT: std::cell::RefCell<String> = "X:/".to_owned().into();
}

/// Tests never start a player: every plugin keeps its patches under one root (`X:/` unless
/// set), at the bundle path last written on this thread.
#[cfg(test)]
pub struct Assumed;

#[cfg(test)]
impl Assumed {
    pub fn set_root(root: &std::path::Path) {
        ROOT.set(root.to_string_lossy().into_owned());
    }
}

#[cfg(test)]
impl Roots for Assumed {
    fn plugin(&mut self, key: &PluginKey, _: bool) -> Result<Plugin, String> {
        let bundle_path = BUNDLES.with_borrow_mut(|bundles| {
            if !key.bundle_path.is_empty() {
                bundles.insert(key.id.clone(), key.bundle_path.clone());
            }
            bundles.get(&key.id).cloned().unwrap_or_default()
        });
        Ok(Plugin {
            format: key.format.clone(),
            id: key.id.clone(),
            bundle_path,
            roots: vec![ROOT.with_borrow(Clone::clone)],
        })
    }

    fn exists(&self, _: &std::path::Path) -> bool {
        true
    }
}
