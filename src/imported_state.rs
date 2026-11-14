use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn store(bytes: &[u8]) -> Result<String, String> {
    let root = crate::storage_paths::directory().ok_or("Application data unavailable")?;
    store_at(&root.join("imported-states"), bytes)
}

pub fn read(sha256: &str) -> Result<Vec<u8>, String> {
    let root = crate::storage_paths::directory().ok_or("Application data unavailable")?;
    read_at(&root.join("imported-states"), sha256)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn read_at(root: &Path, sha256: &str) -> Result<Vec<u8>, String> {
    if !valid_digest(sha256) {
        return Err("Invalid imported state digest".into());
    }
    let path = root.join(format!("{sha256}.state"));
    if std::fs::metadata(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len()
        > 16 * 1024 * 1024
    {
        return Err("Imported state is too large".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.is_empty() || digest(&bytes) != sha256 {
        return Err("Imported state checksum mismatch".into());
    }
    Ok(bytes)
}

pub(crate) fn store_at(root: &Path, bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
        return Err("Invalid imported state size".into());
    }
    let id = digest(bytes);
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let path = root.join(format!("{id}.state"));
    if path.exists() {
        read_at(root, &id)?;
        return Ok(id);
    }
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if let Err(error) = file.persist_noclobber(path) {
        if error.error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error.to_string());
        }
        read_at(root, &id)?;
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistent_state_is_deduplicated_and_corruption_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let id = store_at(dir.path(), &[1, 2, 3]).unwrap();
        assert_eq!(store_at(dir.path(), &[1, 2, 3]).unwrap(), id);
        assert_eq!(read_at(dir.path(), &id).unwrap(), [1, 2, 3]);
        std::fs::write(dir.path().join(format!("{id}.state")), [4]).unwrap();
        assert!(read_at(dir.path(), &id).is_err());
        assert!(read_at(dir.path(), "../escape").is_err());
    }
}
