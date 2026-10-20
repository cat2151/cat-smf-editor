use super::*;

#[test]
fn opening_and_recording_merge_spellings_and_keep_latest_metadata() {
    let root = tempfile::tempdir().unwrap();
    let song = root.path().join("song.mid");
    std::fs::write(&song, []).unwrap();
    let canonical = std::fs::canonicalize(&song).unwrap();
    let entry = |path, opened, title: &str| Entry {
        path,
        opened,
        title: title.into(),
        seconds: 12.0,
        tracks: 3,
    };
    let path = root.path().join("recent-files.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&vec![
            entry(song.clone(), 1, "old"),
            entry(canonical, 2, "latest"),
            entry(root.path().join("other.mid"), 0, "other"),
        ])
        .unwrap(),
    )
    .unwrap();
    let mut history = History::open(path.clone());
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].title, "latest");
    assert_eq!(history.entries[0].seconds, 12.0);
    let music = Music {
        format: 0,
        division: 480,
        tracks: vec![],
    };
    history.record(&song, &music);
    assert!(history.error.is_none());
    let restored = History::open(path);
    assert_eq!(restored.entries.len(), 2);
    assert_eq!(restored.entries[0].path, song);
    assert_eq!(restored.entries[1].title, "other");
}

#[cfg(windows)]
#[test]
fn missing_windows_paths_merge_verbatim_case_separators_and_unc() {
    assert_eq!(
        identity(Path::new("X:/Music/Song.mid")),
        identity(Path::new(r"\\?\x:\music\song.mid"))
    );
    assert_eq!(
        identity(Path::new(r"\\server\share\song.mid")),
        identity(Path::new(r"\\?\UNC\server\share\song.mid"))
    );
    assert_ne!(
        identity(Path::new("X:/a/song.mid")),
        identity(Path::new("X:/b/song.mid"))
    );
}
