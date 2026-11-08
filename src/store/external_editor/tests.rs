use super::*;

#[test]
fn registration_is_read_lazily_once_and_save_updates_the_cached_value() {
    let directory = tempfile::tempdir().unwrap();
    let mut editor = Editor::default();
    assert!(!editor.checked);
    assert!(editor.executable.is_none());
    let executable = std::env::current_exe().unwrap();
    let path = directory.path().join("external-editor.json");
    std::fs::write(&path, serde_json::to_vec(&Some(&executable)).unwrap()).unwrap();
    editor.load_once(Some(directory.path())).unwrap();
    assert_eq!(editor.executable, Some(executable.clone()));
    std::fs::write(&path, "broken").unwrap();
    editor.load_once(Some(directory.path())).unwrap();
    assert_eq!(editor.executable, Some(executable));
    editor.save("").unwrap();
    assert!(editor.executable.is_none());
    assert!(Editor::open(directory.path()).unwrap().executable.is_none());
}

#[test]
fn unreadable_registration_is_checked_once_and_can_be_replaced() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("external-editor.json"), "broken").unwrap();
    let mut editor = Editor::default();
    assert!(editor.load_once(Some(directory.path())).is_err());
    editor.load_once(Some(directory.path())).unwrap();
    editor.save("").unwrap();
    assert!(Editor::open(directory.path()).unwrap().executable.is_none());
}

#[test]
fn launch_waits_for_the_next_native_frame_and_does_not_repeat() {
    let mut editor = Editor::default();
    editor.request(PathBuf::from("X:/missing/settings.toml"), 12);
    editor.launch_pending(12).unwrap();
    editor.launch_pending(12).unwrap();
    assert!(editor.pending.is_some());
    assert!(editor.launch_pending(13).is_err());
    assert!(editor.pending.is_none());
    editor.launch_pending(14).unwrap();
}
