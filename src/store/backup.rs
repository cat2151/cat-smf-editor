//! Unbounded, exclusive-create backups of the SMF being replaced.
use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
};

pub(super) fn save(directory: &Path, source: &Path, bytes: &[u8]) -> io::Result<()> {
    save_at(directory, source, bytes, &timestamp())
}

fn save_at(directory: &Path, source: &Path, bytes: &[u8], stamp: &str) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let stem = source
        .file_stem()
        .ok_or_else(|| io::Error::other("SMF filename is missing"))?;
    let mut sequence = 1u64;
    loop {
        let mut name = OsString::from(stem);
        name.push(format!("_{stamp}"));
        if sequence > 1 {
            name.push(format!(".{sequence}"));
        }
        if let Some(extension) = source.extension() {
            name.push(".");
            name.push(extension);
        }
        let path = directory.join(name);
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                sequence = sequence
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("Backup sequence exhausted"))?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        return Ok(());
    }
}

fn timestamp() -> String {
    // SYSTEMTIME is eight WORDs: year, month, weekday, day, hour, minute, second, ms.
    let mut local = [0u16; 8];
    // SAFETY: GetLocalTime writes exactly one SYSTEMTIME into this aligned buffer.
    unsafe { GetLocalTime(local.as_mut_ptr()) };
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        local[0], local[1], local[3], local[4], local[5], local[6]
    )
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLocalTime(time: *mut u16);
}

#[cfg(test)]
mod tests;
