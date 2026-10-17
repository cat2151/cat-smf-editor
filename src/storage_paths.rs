//! Persistent application data and disposable, regenerable files have separate roots.
use std::path::{Path, PathBuf};

pub fn directory() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .filter(|p| !p.is_empty())
        .map(|root| PathBuf::from(root).join("cat-smf-editor"))
}

pub fn cache(root: &Path) -> PathBuf {
    root.join("cache")
}

pub fn cache_directory() -> Option<PathBuf> {
    directory().map(|root| cache(&root))
}
