//! Write a replacement completely before touching the previously loaded MIDI file.
use std::{
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn replace_with_backup(
    path: &Path,
    bytes: &[u8],
    expected: &[u8],
    backup: &Path,
) -> io::Result<()> {
    replace_after(path, bytes, expected, || {
        super::backup::save(backup, path, expected)
    })
}

fn replace_after(
    path: &Path,
    bytes: &[u8],
    expected: &[u8],
    before_replace: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_file_name(format!(
        ".cat-smf-save-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if std::fs::read(path)? != expected {
            return Err(io::Error::other(
                "MIDI file changed since it was loaded; refusing to overwrite",
            ));
        }
        before_replace()?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
