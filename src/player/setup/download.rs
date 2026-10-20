//! Download into a private staging directory; publish the executable last.
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

pub const URL: &str = "https://github.com/cat2151/cat-plugin-player/releases/download/v0.1.0/cat-plugin-player-0.1.0-windows-x64.zip";
const LICENSE_NAME: &str = "cat-plugin-player-LICENSE.txt";
const LICENSE_DIRECTORY: &str = "cat-smf-editor-licenses";
const EXE: &str = "cat-plugin-player.exe";
const MAX_ZIP: u64 = 256 * 1024 * 1024;

pub fn install(directory: &Path) -> Result<PathBuf, String> {
    if directory.join(EXE).is_file() {
        return Ok(directory.join(EXE));
    }
    let desktop = dirs::desktop_dir().ok_or("Desktop directory is unavailable")?;
    install_inner(directory, &desktop).map_err(|e| format!("cat-plugin-player: {e}"))
}

fn install_inner(directory: &Path, desktop: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target = directory.join(EXE);
    if target.is_file() {
        return Ok(target);
    }
    let staging_dir = tempfile::Builder::new()
        .prefix(".cat-plugin-player-")
        .tempdir_in(directory)?;
    let staging = staging_dir.path();
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .new_agent();
    let mut response = agent.get(URL).call()?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_ZIP + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ZIP {
        return Err("download exceeds size limit".into());
    }
    unpack(&bytes, staging)?;
    publish(directory, staging, desktop)
}

fn publish(
    directory: &Path,
    staging: &Path,
    desktop: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let target = directory.join(EXE);
    validate_player(&staging.join(EXE))?;
    preserve_license(&staging.join("LICENSE"), desktop)?;
    // Windows persist_noclobber uses a non-replacing move, including on FAT/exFAT.
    let mut executable = tempfile::NamedTempFile::new_in(directory)?;
    std::io::copy(
        &mut fs::File::open(staging.join(EXE))?,
        executable.as_file_mut(),
    )?;
    executable.flush()?;
    match executable.persist_noclobber(&target) {
        Ok(_) => {}
        Err(error)
            if error.error.kind() == std::io::ErrorKind::AlreadyExists && target.is_file() => {}
        Err(error) => return Err(error.error.into()),
    }
    Ok(target)
}

/// Group notices in one Desktop folder without overwriting different content.
fn preserve_license(source: &Path, desktop: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let bytes = fs::read(source)?;
    let directory = desktop.join(LICENSE_DIRECTORY);
    fs::create_dir_all(&directory)?;
    let mut target = directory.join(LICENSE_NAME);
    match fs::read(&target) {
        Ok(existing) if existing == bytes => return Ok(target),
        Ok(_) => {
            let hash = format!("{:x}", Sha256::digest(&bytes));
            target = directory.join(format!("cat-plugin-player-LICENSE-{}.txt", &hash[..16]));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut notice = tempfile::NamedTempFile::new_in(&directory)?;
    notice.write_all(&bytes)?;
    notice.flush()?;
    match notice.persist_noclobber(&target) {
        Ok(_) => Ok(target),
        Err(_) if fs::read(&target).is_ok_and(|existing| existing == bytes) => Ok(target),
        Err(error) => Err(error.error.into()),
    }
}

fn validate_player(executable: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new(executable)
        .arg("--help")
        .output()?;
    let help = String::from_utf8_lossy(&output.stdout);
    if !output.status.success()
        || ["--resident", "--restore-state", "--patch-roots"]
            .iter()
            .any(|flag| !help.contains(flag))
    {
        return Err("Published cat-plugin-player is too old for this editor (requires --resident, --restore-state and --patch-roots). A newer player release is required.".into());
    }
    Ok(())
}

fn unpack(bytes: &[u8], directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    // The release contract has exactly these root files. Never extract arbitrary paths.
    for name in ["LICENSE", EXE] {
        let mut entry = zip.by_name(name)?;
        if entry.is_dir() || entry.size() > MAX_ZIP {
            return Err("invalid release entry".into());
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?;
        std::io::copy(&mut entry, &mut output)?;
        output.flush()?;
        if name == EXE {
            let data = fs::read(directory.join(name))?;
            if !data.starts_with(b"MZ") {
                return Err("release is not a Windows executable".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
