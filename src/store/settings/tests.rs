use crate::store::settings::Settings;

#[test]
fn creates_template_once_and_preserves_user_comments_and_reports_bad_values() {
    let root = std::env::temp_dir().join(format!("cat-smf-settings-{}", std::process::id()));
    let path = root.join("settings.toml");
    assert_eq!(
        Settings::open(&root).unwrap().window,
        [0.0, 0.0, 1800.0, 900.0]
    );
    assert!(!root.join("projects").exists());
    let custom = "# keep my comment\nwindow = [-10, 25, 1200, 700] # size\n";
    std::fs::write(&path, custom).unwrap();
    assert_eq!(
        Settings::open(&root).unwrap().window,
        [-10.0, 25.0, 1200.0, 700.0]
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), custom);
    for invalid in [
        "window = [0, 0, 0, 900]",
        "window = [0, 0, inf, 900]",
        "window = [0, 0, 1]",
        "status = 1",
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(Settings::open(&root).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}
