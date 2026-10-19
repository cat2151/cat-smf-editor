use super::*;

#[test]
fn bundled_dictionary_matches_romaji_and_preserves_literal_search() {
    let a = app();
    let ctx = egui::Context::default();
    let commands = commands::candidates(&a, &ctx);
    let mut search = commands::SearchQuery::default();
    for (query, id) in [
        ("hiraku", "open"),
        ("HIRAKU", "open"),
        ("hira", "open"),
        ("hirak", "open"),
        ("hozon", "save"),
        ("syuuryou", "quit"),
        ("開く", "open"),
        ("保存", "save"),
        ("open", "open"),
        ("CTRL+O", "open"),
        ("ctrl+shift+n", "add-track"),
    ] {
        let result = filter(&commands, query, &mut search);
        assert!(
            result.iter().any(|&i| commands[i].id == id),
            "{query}: {id}"
        );
    }
    for query in ["[", ".*", "(", "no-such-operation", "😀"] {
        assert!(filter(&commands, query, &mut search).is_empty(), "{query}");
    }
    assert_eq!(filter(&commands, "", &mut search).len(), commands.len());
    assert!(!filter(&commands, "a", &mut search).is_empty());
}

#[test]
fn romaji_search_keyboard_selection_execute_and_cancel() {
    let mut a = app();
    let ctx = egui::Context::default();
    open(&mut a, &ctx);
    query(&mut a, &ctx, "hiraku");
    let results = filter(&a.palette.commands, &a.palette.query, &mut a.palette.search);
    assert_eq!(a.palette.commands[results[0]].id, "open");
    press(&mut a, &ctx, Key::ArrowDown);
    press(&mut a, &ctx, Key::ArrowUp);
    assert_eq!(a.palette.selected, 0);
    press(&mut a, &ctx, Key::Escape);
    assert!(!a.palette.is_open());
    assert!(!a.file_open_requested);
    open(&mut a, &ctx);
    query(&mut a, &ctx, "hiraku");
    press(&mut a, &ctx, Key::Enter);
    assert!(!a.palette.is_open());
    assert!(a.file_open_requested);
}
