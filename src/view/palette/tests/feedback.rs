use super::*;

#[test]
fn basic_english_finds_help_play_and_quit_without_bypassing_confirmation() {
    let mut a = app();
    let c = egui::Context::default();
    open(&mut a, &c);
    let matching = |q: &str| {
        filter(
            &a.palette.commands,
            q,
            &mut commands::SearchQuery::default(),
        )
        .into_iter()
        .map(|i| a.palette.commands[i].id.clone())
        .collect::<Vec<_>>()
    };
    assert!(matching("HELP").contains(&"help".into()));
    assert!(matching("play").contains(&"play-last".into()));
    assert_eq!(matching("quit"), vec!["quit"]);
    assert_eq!(matching("EXIT"), vec!["quit"]);
    let help = a.palette.commands.iter().find(|c| c.id == "help").unwrap();
    assert!(help.key_label().contains("F1"));
    query(&mut a, &c, "help");
    press(&mut a, &c, Key::Enter);
    assert!(a.help_open);
    press(&mut a, &c, Key::Escape);
    // Untitled edited data must still receive the existing save/discard dialog.
    a.loaded.as_mut().unwrap().music.tracks[1].events[0].data[0] = 63;
    open(&mut a, &c);
    query(&mut a, &c, "exit");
    press(&mut a, &c, Key::Enter);
    assert!(matches!(
        a.config.navigation.departure,
        Some(super::super::super::files::Action::Close)
    ));
    assert!(!a.config.navigation.close_approved);
    assert!(a.loaded.is_some());
    open(&mut a, &c);
    assert!(filter(&a.palette.commands, "quit", &mut a.palette.search).is_empty());
    press(&mut a, &c, Key::Escape);
    press(&mut a, &c, Key::Escape);
    assert!(a.config.navigation.departure.is_none());
    assert!(a.loaded.is_some());
}

#[test]
fn centered_opening_width_and_query_stay_fixed_while_results_shrink() {
    let mut a = app();
    let c = egui::Context::default();
    open(&mut a, &c);
    frame(&mut a, &c, vec![]);
    let rect = || c.memory(|m| m.area_rect(egui::Id::new("command-palette")).unwrap());
    let full = rect();
    let query_rect = || {
        c.read_response(egui::Id::new("command-palette-query"))
            .unwrap()
            .rect
    };
    let input = query_rect();
    assert!(full.top() < 25.0, "{full:?}");
    assert!(full.bottom() <= 900.0, "{full:?}");
    assert!(full.width() < 1000.0, "{full:?}");
    assert!((full.center().x - 750.0).abs() <= 1.0, "{full:?}");
    query(&mut a, &c, "quit");
    frame(&mut a, &c, vec![]);
    let one = rect();
    assert_eq!(one.min, full.min);
    assert_eq!(one.width(), full.width());
    assert!(one.height() < full.height(), "{one:?} / {full:?}");
    assert_eq!(query_rect(), input);
    a.palette.query.clear();
    query(&mut a, &c, "no-such-operation");
    frame(&mut a, &c, vec![]);
    let empty = rect();
    assert_eq!(empty.min, full.min);
    assert_eq!(empty.width(), full.width());
    assert!(empty.height() < one.height(), "{empty:?} / {one:?}");
    assert_eq!(query_rect(), input);
    a.palette.query.clear();
    frame(&mut a, &c, vec![]);
    frame(&mut a, &c, vec![]);
    assert_eq!(rect(), full);
    press(&mut a, &c, Key::Escape);
    open(&mut a, &c);
    frame(&mut a, &c, vec![]);
    assert_eq!(rect(), full);
    // A new opening measures its own candidate list, rather than retaining the
    // wider geometry of the previous screen.
    press(&mut a, &c, Key::Escape);
    a.config.navigation.departure = Some(super::super::super::files::Action::New);
    open(&mut a, &c);
    frame(&mut a, &c, vec![]);
    let confirmation = rect();
    assert!(confirmation.width() < full.width());
    assert!((confirmation.center().x - 750.0).abs() <= 1.0);
}
