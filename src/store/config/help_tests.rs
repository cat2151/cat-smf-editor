use super::*;

#[test]
fn absent_help_defaults_closed_and_geometry_round_trip_preserves_existing_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let mut original = serde_json::to_value(Status::default()).unwrap();
    original.as_object_mut().unwrap().remove("help_open");
    original.as_object_mut().unwrap().remove("help_window");
    original["analysis_open"] = true.into();
    original["analysis_window"] = serde_json::json!([100.0, 120.0, 760.0, 260.0]);
    original["pattern_windows"] = serde_json::json!({"Scene": [30.0, 40.0, 700.0, 400.0]});
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    let mut store = Store::open(path.clone());
    assert!(!store.help_open);
    assert_eq!(store.help_window, None);
    store.help_open = true;
    store.help_window = Some([-1200.0, 80.0, 600.0, 700.0]);
    store.persist();
    let reopened = Store::open(path.clone());
    assert!(reopened.help_open);
    assert_eq!(reopened.help_window, store.help_window);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for (key, value) in original.as_object().unwrap() {
        assert_eq!(&saved[key], value, "existing field {key}");
    }
}

#[test]
fn invalid_help_geometry_is_discarded_without_losing_open_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    let status = Status {
        help_open: true,
        help_window: Some([1.0, 2.0, 0.0, -4.0]),
        ..Default::default()
    };
    std::fs::write(&path, serde_json::to_vec(&status).unwrap()).unwrap();
    let store = Store::open(path);
    assert!(store.help_open);
    assert_eq!(store.help_window, None);
}

#[test]
fn real_settings_backup_round_trip_when_requested() {
    let Some(source) = std::env::var_os("CAT_SMF_HELP_STATUS_FIXTURE") else {
        return;
    };
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&source).unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("status.json");
    std::fs::copy(&source, &path).unwrap();
    let startup = std::path::Path::new(&source).with_file_name("original-startup.json");
    if startup.exists() {
        std::fs::copy(startup, dir.path().join("startup.json")).unwrap();
    }
    let mut store = Store::open(path.clone());
    assert!(store.error.is_none(), "{:?}", store.error);
    store.help_open = true;
    store.help_window = Some([60.0, 70.0, 620.0, 680.0]);
    store.persist();
    assert!(store.error.is_none(), "{:?}", store.error);
    let reopened = Store::open(path.clone());
    assert!(reopened.help_open);
    assert_eq!(reopened.help_window, store.help_window);
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for (key, value) in original.as_object().unwrap() {
        assert_eq!(&saved[key], value, "real existing field {key}");
    }
}
