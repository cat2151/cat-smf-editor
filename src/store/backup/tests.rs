use super::*;

#[test]
fn collision_sequence_preserves_every_generation_and_extension() {
    let root = std::env::temp_dir().join(format!("cat-smf-backup-names-{}", std::process::id()));
    let stamp = "20261017123456";
    for bytes in [b"first".as_slice(), b"second", b"third"] {
        save_at(&root, Path::new("日本語.foo.mid"), bytes, stamp).unwrap();
    }
    for (name, bytes) in [
        ("日本語.foo_20261017123456.mid", b"first".as_slice()),
        ("日本語.foo_20261017123456.2.mid", b"second"),
        ("日本語.foo_20261017123456.3.mid", b"third"),
    ] {
        assert_eq!(std::fs::read(root.join(name)).unwrap(), bytes);
        std::fs::remove_file(root.join(name)).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn backup_failure_preserves_previous_save_and_retry_keeps_its_exact_bytes() {
    let root = std::env::temp_dir().join(format!("cat-smf-backup-failure-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("song.mid");
    let backup = root.join("backup");
    std::fs::write(&source, b"old").unwrap();
    std::fs::write(&backup, b"blocked").unwrap();
    assert!(
        crate::store::saved_write::replace_with_backup(&source, b"new", b"old", &backup).is_err()
    );
    assert_eq!(std::fs::read(&source).unwrap(), b"old");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    std::fs::remove_file(&backup).unwrap();
    // An externally changed saved copy must be rejected before creating any backup.
    assert!(
        crate::store::saved_write::replace_with_backup(&source, b"new", b"external", &backup)
            .is_err()
    );
    assert!(!backup.exists());
    crate::store::saved_write::replace_with_backup(&source, b"new", b"old", &backup).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), b"new");
    let entries = std::fs::read_dir(&backup)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(entries.len(), 1);
    let path = entries[0].path();
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("song_") && name.ends_with(".mid"));
    assert_eq!(name.len(), "song_".len() + 14 + ".mid".len());
    assert!(name[5..19].bytes().all(|b| b.is_ascii_digit()));
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(backup).unwrap();
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir(root).unwrap();
}
