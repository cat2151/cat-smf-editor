//! Only the document identity belongs on the pre-audio restore path (ADR 0011).
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub last_mid: Option<PathBuf>,
}

pub fn read(status_path: &Path) -> Result<Status, String> {
    let path = status_path.with_file_name("startup.json");
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Status::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

pub fn save(status_path: &Path, last_mid: Option<PathBuf>) -> Result<(), String> {
    let path = status_path.with_file_name("startup.json");
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(&Status { last_mid })?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, &path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result.map_err(|e| format!("{}: {e}", path.display()))
}
