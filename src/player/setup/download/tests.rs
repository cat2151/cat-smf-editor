use super::*;
use zip::{write::SimpleFileOptions, ZipWriter};

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn extracts_only_release_files_and_preserves_existing_executable() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = archive(&[("LICENSE", b"MIT"), (EXE, b"MZtest"), ("../escape", b"bad")]);
    unpack(&bytes, temp.path()).unwrap();
    assert_eq!(fs::read(temp.path().join(EXE)).unwrap(), b"MZtest");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    // No network access when a player already exists.
    assert_eq!(install(temp.path()).unwrap(), temp.path().join(EXE));
    assert_eq!(fs::read(temp.path().join(EXE)).unwrap(), b"MZtest");
}

#[test]
fn rejects_incomplete_or_non_executable_releases() {
    for bytes in [
        archive(&[("LICENSE", b"MIT")]),
        archive(&[("LICENSE", b"MIT"), (EXE, b"html")]),
    ] {
        let temp = tempfile::tempdir().unwrap();
        assert!(unpack(&bytes, temp.path()).is_err());
    }
}

#[test]
fn desktop_license_keeps_existing_content_and_reuses_identical_notice() {
    let desktop = tempfile::tempdir().unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    fs::write(source.path(), b"player notice").unwrap();
    let first = preserve_license(source.path(), desktop.path()).unwrap();
    assert_eq!(first.file_name().unwrap(), LICENSE_NAME);
    assert_eq!(
        preserve_license(source.path(), desktop.path()).unwrap(),
        first
    );
    fs::write(&first, b"existing desktop document").unwrap();
    let other = preserve_license(source.path(), desktop.path()).unwrap();
    assert_ne!(first, other);
    assert!(other
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("cat-plugin-player-LICENSE-"));
    assert_eq!(fs::read(&first).unwrap(), b"existing desktop document");
    assert_eq!(fs::read(&other).unwrap(), b"player notice");
    assert_eq!(
        preserve_license(source.path(), desktop.path()).unwrap(),
        other
    );
    assert_eq!(fs::read_dir(desktop.path()).unwrap().count(), 2);
}

#[test]
#[ignore = "requires the sibling player's current release build"]
fn compatible_local_release_extracts_publishes_and_preserves_editor_license() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cat-plugin-player");
    let executable = fs::read(root.join("target/release").join(EXE)).unwrap();
    let license = fs::read(root.join("LICENSE")).unwrap();
    let bytes = archive(&[(EXE, &executable), ("LICENSE", &license)]);
    let destination = tempfile::tempdir().unwrap();
    let desktop = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir_in(destination.path()).unwrap();
    fs::write(destination.path().join("LICENSE"), b"editor license").unwrap();
    unpack(&bytes, staging.path()).unwrap();
    let installed = publish(destination.path(), staging.path(), desktop.path()).unwrap();
    validate_player(&installed).unwrap();
    assert_eq!(fs::read(installed).unwrap(), executable);
    assert_eq!(
        fs::read(desktop.path().join(LICENSE_NAME)).unwrap(),
        license
    );
    assert_eq!(
        fs::read(destination.path().join("LICENSE")).unwrap(),
        b"editor license"
    );
}

#[test]
#[ignore = "downloads the real published release into a temporary directory"]
fn published_release_is_checked_before_installing() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("LICENSE"), b"editor license").unwrap();
    let desktop = tempfile::tempdir().unwrap();
    let result = install_inner(temp.path(), desktop.path()).map_err(|e| e.to_string());
    if let Err(error) = result {
        assert!(error.contains("too old for this editor"), "{error}");
        assert!(!temp.path().join(EXE).exists());
        assert_eq!(fs::read_dir(desktop.path()).unwrap().count(), 0);
        assert_eq!(
            fs::read(temp.path().join("LICENSE")).unwrap(),
            b"editor license"
        );
        eprintln!("Published release was correctly rejected: {error}");
        return;
    }
    let exe = result.unwrap();
    assert_eq!(
        fs::read(temp.path().join("LICENSE")).unwrap(),
        b"editor license"
    );
    assert!(desktop.path().join(LICENSE_NAME).is_file());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    let output = std::process::Command::new(exe)
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("--resident"));
}
