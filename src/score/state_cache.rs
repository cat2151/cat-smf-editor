//! Regenerable patch states, persisted atomically under the application's cache root.
use crate::{
    chain_settings::PluginState,
    patch_reference::{PatchReference, Source},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_STATE: usize = 16 * 1024 * 1024;
const GENERATOR_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    version: u32,
    reference: PatchReference,
    fingerprint: String,
    state: Vec<u8>,
    checksum: String,
}

#[derive(Clone)]
pub struct Cache {
    directory: Option<PathBuf>,
    used: BTreeMap<String, Entry>,
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn key(reference: &PatchReference) -> Result<String, String> {
    reference.validate()?;
    serde_json::to_vec(&(
        &reference.plugin.format,
        &reference.plugin.id,
        &reference.plugin.bundle_path,
        &reference.source,
    ))
    .map(|bytes| hash(&bytes))
    .map_err(|e| e.to_string())
}

fn stamp(path: &Path, output: &mut Vec<u8>) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    output.extend(path.to_string_lossy().as_bytes());
    output.extend(metadata.len().to_le_bytes());
    let modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    output.extend(modified.as_nanos().to_le_bytes());
    if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        children.sort();
        for child in children {
            if std::fs::symlink_metadata(&child)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Symlinked plugin bundles cannot be cached safely".into());
            }
            stamp(&child, output)?;
        }
    }
    Ok(())
}

fn fingerprint(reference: &PatchReference) -> Result<String, String> {
    let mut bytes = GENERATOR_VERSION.to_le_bytes().to_vec();
    stamp(Path::new(&reference.plugin.bundle_path), &mut bytes)?;
    let source = match &reference.source {
        Source::Catalog { file, .. } => file,
        Source::Effect { path, .. } => path,
    };
    stamp(source, &mut bytes)?;
    Ok(hash(&bytes))
}

impl Cache {
    /// Keep states resolved by idle work without replacing newer foreground entries.
    pub fn absorb(&mut self, other: Self) {
        for (key, entry) in other.used {
            self.used.entry(key).or_insert(entry);
        }
    }

    pub fn new(application_data: Option<PathBuf>) -> Self {
        Self {
            directory: application_data
                .map(|p| crate::storage_paths::cache(&p).join("patch-states-v1")),
            used: BTreeMap::new(),
        }
    }

    /// No player discovery or catalog scan occurs on a hit, including disk hits.
    pub fn resolve(
        &mut self,
        reference: &PatchReference,
        generate: impl FnOnce(&PatchReference) -> Result<Vec<u8>, String>,
    ) -> Result<PluginState, String> {
        let id = key(reference)?;
        let current = fingerprint(reference).ok();
        let valid = |entry: &Entry| {
            entry.version == GENERATOR_VERSION
                && key(&entry.reference).ok().as_ref() == Some(&id)
                && current.as_ref() == Some(&entry.fingerprint)
                && !entry.state.is_empty()
                && entry.state.len() <= MAX_STATE
                && hash(&entry.state) == entry.checksum
        };
        if !self.used.get(&id).is_some_and(valid) {
            let disk = self
                .directory
                .as_ref()
                .and_then(|dir| {
                    let path = dir.join(format!("{id}.json"));
                    if std::fs::metadata(&path).ok()?.len() > (MAX_STATE * 5) as u64 {
                        return None;
                    }
                    serde_json::from_slice::<Entry>(&std::fs::read(path).ok()?).ok()
                })
                .filter(valid);
            if let Some(entry) = disk {
                self.used.insert(id.clone(), entry);
            } else {
                let state = generate(reference)?;
                if current.is_some() && fingerprint(reference).ok() != current {
                    return Err("Patch or plugin changed during state generation".into());
                }
                self.remember(reference, state)?;
            }
        }
        let entry = self
            .used
            .get(&id)
            .ok_or("Generated state was not retained")?;
        Ok(PluginState {
            reference: Some(reference.clone()),
            plugin: reference.plugin.clone(),
            state: entry.state.clone(),
            preset_name: Some(reference.display.clone()),
            bypass: false,
        })
    }

    pub fn remember(&mut self, reference: &PatchReference, state: Vec<u8>) -> Result<(), String> {
        let id = key(reference)?;
        if state.is_empty() || state.len() > MAX_STATE {
            return Err("Invalid generated state size".into());
        }
        let fingerprint = fingerprint(reference).unwrap_or_default();
        self.used.insert(
            id,
            Entry {
                version: GENERATOR_VERSION,
                reference: reference.clone(),
                fingerprint,
                checksum: hash(&state),
                state,
            },
        );
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };
        std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        for (id, entry) in &self.used {
            if entry.fingerprint.is_empty() {
                continue;
            }
            if fingerprint(&entry.reference).ok().as_ref() != Some(&entry.fingerprint) {
                continue;
            }
            let path = directory.join(format!("{id}.json"));
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let temp = directory.join(format!(
                ".{id}-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let bytes = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
            let mut created = false;
            let result = (|| {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp)?;
                created = true;
                file.write_all(&bytes)?;
                file.sync_all()?;
                drop(file);
                std::fs::rename(&temp, &path)
            })();
            if let Err(e) = result {
                if created {
                    let _ = std::fs::remove_file(&temp);
                }
                return Err(format!("State cache {}: {e}", path.display()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_cache_tests.rs"]
mod tests;
