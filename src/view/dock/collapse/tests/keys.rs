//! Folding from the keyboard: Ctrl+F6 for the focused pane, the palette for any pane.
use super::*;
use crate::view::commands::{self, Screen};
use crate::view::palette::tests::{listed, open, press, query};

const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);

#[test]
fn ctrl_f6_folds_the_focused_pane_and_the_focus_moves_on() {
    let (mut a, c) =
        app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }, { view = "tempo_curve" }]"#);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    key(&mut a, &c, dock(), Key::F6, CTRL);
    assert!(folded(&a, ViewKind::Mixer));
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    redraw(&mut a, &c);
    key(&mut a, &c, dock(), Key::F6, CTRL);
    assert!(folded(&a, ViewKind::TempoCurve));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    // F6 alone still moves without folding.
    redraw(&mut a, &c);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert!(!folded(&a, ViewKind::PianoRoll));
}

#[test]
fn ctrl_f6_folds_a_pane_of_the_main_window() {
    let (mut a, c) = app(r#"view = "mixer""#);
    a.layout.spec = crate::store::layout::parse(
        "[root]
row = [{ view = \"event_list\" }, { view = \"score\" }]",
    )
    .unwrap();
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    key(&mut a, &c, ROOT, Key::F6, CTRL);
    assert!(folded(&a, ViewKind::EventList));
    assert_eq!(focus::focused(&a), ViewKind::Score);
}

#[test]
fn the_palette_folds_and_opens_each_pane_of_the_focused_window() {
    let (mut a, c) =
        app(r#"column = [{ view = "mixer" }, { view = "tempo_curve" }, { view = "piano" }]"#);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    let choose = |a: &mut App, q: &str, id: &str| {
        open(a, &c);
        query(a, &c, q);
        let row = listed(a)
            .iter()
            .position(|listed| listed == id)
            .unwrap_or_else(|| panic!("{q}: {:?}", listed(a)));
        for _ in 0..row {
            press(a, &c, Key::ArrowDown);
        }
        press(a, &c, Key::Enter);
        assert!(!a.palette.is_open());
    };
    // The piano takes no focus, so the palette is its only key; romaji finds it too.
    choose(&mut a, "piano oritatami", "pane.fold-Piano");
    assert!(folded(&a, ViewKind::Piano));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    choose(&mut a, "mixer の折りたたみ", "pane.fold-Mixer");
    assert!(folded(&a, ViewKind::Mixer));
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    // A folded pane cannot take the focus, yet it opens from the palette.
    choose(&mut a, "mixer の折りたたみ", "pane.fold-Mixer");
    assert!(!folded(&a, ViewKind::Mixer));
    choose(&mut a, "piano の折りたたみ", "pane.fold-Piano");
    assert!(!folded(&a, ViewKind::Piano));
}

#[test]
fn a_lone_pane_offers_no_fold() {
    let (a, _c) = app(r#"view = "mixer""#);
    for id in ["pane.fold", "pane.fold-EventList"] {
        assert!(
            !commands::definitions(&a, Screen::Main)
                .iter()
                .any(|cmd| cmd.id == id),
            "{id}"
        );
    }
}

/// Every screen named in the palette definitions, and the global one.
fn screens() -> Vec<Screen> {
    let text = include_str!("../../../../../config/command-palette.toml");
    let table: toml::Table = toml::from_str(text).unwrap();
    let mut screens = vec![Screen::Global];
    for name in table["screens"].as_table().unwrap().keys() {
        screens.push(toml::Value::String(name.clone()).try_into().unwrap());
    }
    screens
}

#[test]
fn ctrl_f6_is_bound_to_nothing_else() {
    let ctrl_f6 = (Key::F6, Modifiers::CTRL);
    let (mut a, c) =
        app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }, { view = "song" }]"#);
    let check = |a: &App| {
        let screens = screens();
        assert!(screens.len() > 40, "{}", screens.len());
        for screen in screens {
            for cmd in commands::definitions(a, screen) {
                if cmd.keys.contains(&ctrl_f6) {
                    assert_eq!(cmd.id, "pane.fold", "{screen:?}");
                }
            }
        }
    };
    check(&a);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    check(&a);
    a.note_mode = true;
    a.layout.spec = crate::store::layout::parse("[root]\nview = \"event_list\"").unwrap();
    check(&a);
}

#[test]
fn the_key_opening_a_folded_panes_view_opens_the_pane_with_the_focus() {
    for (view, open) in [(ViewKind::PianoRoll, Key::R), (ViewKind::Mixer, Key::M)] {
        let (mut a, c) = app(r#"column = [{ view = "piano_roll" }, { view = "mixer" }]"#);
        toggle(&mut a, view);
        redraw(&mut a, &c);
        assert!(folded(&a, view), "{view:?}");
        key(&mut a, &c, ROOT, open, Modifiers::NONE);
        assert!(!folded(&a, view), "{view:?}");
        assert_eq!(focus::focused(&a), view);
    }
}
