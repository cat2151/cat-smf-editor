use super::*;

#[test]
fn list_navigation_repeats_clamps_and_preserves_query_and_owner() {
    let mut a = app();
    let c = egui::Context::default();
    open(&mut a, &c);
    let count = listed(&mut a).len();
    assert!(count > 20);
    let track = a.track;
    for (k, m, expected) in [
        (Key::PageDown, Modifiers::NONE, 10),
        (Key::PageDown, Modifiers::NONE, 20),
        (Key::PageUp, Modifiers::NONE, 10),
        (Key::End, Modifiers::CTRL, count - 1),
        (Key::ArrowDown, Modifiers::NONE, count - 1),
        (Key::Home, Modifiers::CTRL, 0),
        (Key::ArrowUp, Modifiers::NONE, 0),
    ] {
        let mut e = key(k, m);
        if let egui::Event::Key { repeat, .. } = &mut e {
            *repeat = true;
        }
        frame(&mut a, &c, vec![e]);
        assert_eq!(a.palette.selected, expected);
        assert!(a.palette.query.is_empty());
        assert_eq!(a.track, track);
    }
    query(&mut a, &c, "quit");
    frame(&mut a, &c, vec![key(Key::End, Modifiers::CTRL)]);
    assert_eq!(a.palette.selected, 0);
    assert_eq!(listed(&mut a).len(), 1);
    frame(&mut a, &c, vec![egui::Event::Text("no-match".into())]);
    frame(&mut a, &c, vec![key(Key::End, Modifiers::CTRL)]);
    assert_eq!(a.palette.selected, 0);
    assert!(listed(&mut a).is_empty());
    press(&mut a, &c, Key::Escape);
    assert!(!a.palette.is_open());
}
