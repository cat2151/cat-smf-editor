use super::*;

#[test]
fn and_terms_match_english_keywords_in_any_order() {
    let a = app();
    let c = egui::Context::default();
    let commands = commands::candidates(&a, &c);
    let mut search = commands::SearchQuery::default();
    for text in [
        "add effect",
        "EFFECT   ADD",
        "追加 effect",
        "add エフェクト",
    ] {
        let results = filter(
            &commands,
            text,
            &mut search,
            &super::super::history::History::default(),
        );
        assert!(
            results.iter().any(|&i| commands[i].id == "effect-add"),
            "{text}"
        );
        assert!(!results.iter().any(|&i| commands[i].id == "effect-remove"));
    }
}

#[test]
fn query_history_wins_over_global_history_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("status.json");
    let mut a = app();
    a.config = crate::store::config::Store::open(path.clone());
    let c = egui::Context::default();
    open(&mut a, &c);
    query(&mut a, &c, "v");
    let candidates = filter(
        &a.palette.commands,
        "v",
        &mut a.palette.search,
        &a.palette.history,
    );
    a.palette.selected = candidates
        .iter()
        .position(|&i| a.palette.commands[i].id == "view-mode")
        .unwrap();
    press(&mut a, &c, Key::Enter);
    assert!(!a.palette.is_open());
    let view = Command::plain(
        Screen::Main,
        "view-mode",
        "Notes / Raw",
        &[Key::V],
        |_, _| {},
    );
    let save = Command::plain(Screen::Main, "save", "保存", &[], |_, _| {});
    // A later selection under another query leads the global list.
    a.palette.history.record("save", &save);
    a.palette.history.save(&a).unwrap();
    let mut b = app();
    b.config = crate::store::config::Store::open(path);
    open(&mut b, &c);
    assert_eq!(b.palette.history.rank("v", &view), (0, 0));
    assert_eq!(b.palette.history.rank("", &save), (1, 0));
    query(&mut b, &c, "v");
    assert_eq!(listed(&mut b).first().unwrap(), "Event list: Notes / Raw");
    press(&mut b, &c, Key::Escape);
    let bytes = std::fs::read(root.path().join("command-palette-history.json")).unwrap();
    open(&mut b, &c);
    press(&mut b, &c, Key::Escape);
    assert_eq!(
        std::fs::read(root.path().join("command-palette-history.json")).unwrap(),
        bytes
    );
}
