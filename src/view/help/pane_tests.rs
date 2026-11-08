//! Help placed in a pane shows the keys of the focused view and never takes the focus.
use crate::app::App;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key};
use eframe::egui::{self, Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
/// A key of the event list's help and of the mixer's.
const MAIN: &str = "移動menu";
const MIXER: &str = "Track / Master選択";
/// Shown only in help's own window, which `Esc` closes.
const CLOSE: &str = "Esc: 閉じる";

fn app(layout: &str) -> App {
    let mut a = crate::view::palette::tests::app();
    a.layout.spec = crate::store::layout::parse(layout).unwrap();
    a
}

const DOCKED: &str = r#"
[root]
row = [{ view = "event_list" }, { view = "mixer" }]

[[window]]
title = "Dock"
rect = [0, 0, 600, 400]
view = "help"
"#;

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

fn shows(c: &egui::Context, id: egui::ViewportId, text: &str) -> bool {
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
        })
        .unwrap_or_default();
    format!("{shapes:?}").contains(text)
}

fn opened(c: &egui::Context) -> bool {
    c.data(|d| {
        d.get_temp::<egui::ViewportBuilder>(egui::Id::new((
            "window-builder",
            crate::view::help::viewport(),
        )))
    })
    .is_some()
}

fn press(a: &mut App, c: &egui::Context, id: egui::ViewportId, pos: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            a,
            c,
            id,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}

#[test]
fn the_placed_help_shows_the_keys_of_the_focused_view() {
    let mut a = app(DOCKED);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), MAIN));
    assert!(!shows(&c, dock(), MIXER));
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), MIXER));
    assert!(!shows(&c, dock(), MAIN));
    key(&mut a, &c, ROOT, Key::Escape, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), MAIN));
    assert!(!opened(&c));
}

#[test]
fn the_help_pane_takes_no_focus_from_a_press_a_key_or_f6() {
    let mut a = app(DOCKED);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    press(&mut a, &c, dock(), egui::pos2(300.0, 200.0));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    // A key typed into a window of only help goes to the focused view.
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(owner(&a, &c), Screen::Mixer);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), MIXER));

    // `F6` in a window passes over its help pane.
    let mut a = app(r#"
[root]
row = [{ view = "event_list" }, { view = "mixer" }, { view = "help" }]
"#);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    for expected in [ViewKind::Mixer, ViewKind::EventList, ViewKind::Mixer] {
        key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
        assert_eq!(focus::focused(&a), expected);
        assert_ne!(a.pane_focus, Some(ViewKind::Help));
    }
    // A press on the help pane at the right end leaves the focus where it was.
    press(&mut a, &c, ROOT, egui::pos2(1450.0, 450.0));
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
}

#[test]
fn f1_does_nothing_while_help_is_placed_and_works_again_once_it_is_taken_out() {
    let mut a = app(DOCKED);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, ROOT, Key::F1, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![egui::Event::Text("?".into())]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!a.help_open);
    assert!(!opened(&c));
    assert!(shows(&c, dock(), MAIN));

    a.layout.spec = crate::store::layout::parse("").unwrap();
    key(&mut a, &c, ROOT, Key::F1, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(a.help_open);
    assert!(opened(&c));
    assert!(shows(&c, crate::view::help::viewport(), MAIN));
    key(
        &mut a,
        &c,
        crate::view::help::viewport(),
        Key::F1,
        Modifiers::NONE,
    );
    assert!(!a.help_open);
}

#[test]
fn a_key_in_a_dock_window_goes_to_its_first_pane_other_than_help() {
    let mut a = app(r#"
[[window]]
title = "Dock"
rect = [0, 0, 600, 400]
column = [{ view = "help" }, { view = "mixer" }]
"#);
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::J, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(owner(&a, &c), Screen::Mixer);
    frame(&mut a, &c, dock(), vec![]);
    assert!(shows(&c, dock(), MIXER));
}

#[test]
fn help_left_open_at_the_last_exit_shows_only_in_its_pane() {
    let mut a = app(DOCKED);
    a.help_open = true;
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!opened(&c));
    assert!(shows(&c, dock(), MAIN));
    a.layout.spec = crate::store::layout::parse("").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c));
}

#[test]
fn only_help_in_its_own_window_says_esc_closes_it() {
    // Tall enough that the last line of the event list's help is drawn.
    let mut a = app(&DOCKED.replace("[0, 0, 600, 400]", "[0, 0, 600, 3000]"));
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), "Ctrl+Shift+P"));
    assert!(!shows(&c, dock(), CLOSE));

    a.layout.spec = crate::store::layout::parse("").unwrap();
    a.help.geometry = Some([0.0, 0.0, 620.0, 3000.0]);
    key(&mut a, &c, ROOT, Key::F1, Modifiers::NONE);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, crate::view::help::viewport(), CLOSE));
}
