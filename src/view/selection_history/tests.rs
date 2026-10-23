use super::*;
#[test]
fn existing_command_format_round_trips_and_unreadable_history_is_preserved() {
    let root = tempfile::tempdir().unwrap();
    let mut app = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    app.config = crate::store::config::Store::open(root.path().join("status.json"));
    let file = root.path().join(SINGER);
    let original = br#"{"queries":{"v":["Main/view-mode"]},"recent":["Main/view-mode"]}"#;
    std::fs::write(&file, original).unwrap();
    let history = History::load(&mut app, SINGER, History::default());
    assert_eq!(history.rank("v", "Main/view-mode"), (0, 0));
    history.save(&app, SINGER).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&file).unwrap()).unwrap(),
        serde_json::from_slice::<serde_json::Value>(original).unwrap()
    );
    std::fs::write(&file, b"broken").unwrap();
    let mut history = History::load(&mut app, SINGER, History::default());
    history.record("", "new");
    history.save(&app, SINGER).unwrap();
    assert!(app.config.error.is_some());
    assert_eq!(std::fs::read(&file).unwrap(), b"broken");
}
