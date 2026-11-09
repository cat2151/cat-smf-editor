//! A tab group's toggle on its line of tab names, what the folded group leaves drawn, and the
//! keys and palette folding and opening it.
use super::*;
use crate::view::dock::tabs;
use crate::view::palette::tests::{listed, open, press, query};

const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);
const GROUP: &str = r#"column = [{ tabs = [{ view = "mixer" }, { view = "tempo_curve" }] }, { view = "piano_roll" }]"#;
const HELP_FIRST: &str =
    r#"column = [{ tabs = [{ view = "help" }, { view = "mixer" }] }, { view = "piano_roll" }]"#;

fn shown(a: &App, view: ViewKind) -> ViewKind {
    tabs::shown(a, &tabs::group(&focus::root(a), view).unwrap())
}

#[test]
fn the_toggle_on_the_tab_line_folds_the_group_in_a_column_to_one_line() {
    let (mut a, c) = app(GROUP);
    assert!(text(&c, dock(), "Master effects").is_some());
    let mixer = text(&c, dock(), "mixer").unwrap();
    let tempo = text(&c, dock(), "tempo_curve").unwrap();
    // One line: the toggle and both tab names share it.
    assert!((mixer.center().y - tempo.center().y).abs() < 2.0);
    let toggle = toggle_of(&c, dock(), "mixer");
    assert!((toggle.y - mixer.center().y).abs() < 6.0);

    frame(&mut a, &c, dock(), click(toggle));
    redraw(&mut a, &c);
    assert!(folded(&a, ViewKind::Mixer));
    assert!(folded(&a, ViewKind::TempoCurve));
    assert!(text(&c, dock(), "Master effects").is_none());
    assert!(text(&c, dock(), "tempo_curve").is_none());
    // The shown tab names the line at the top; the roll below took the rest.
    let line = text(&c, dock(), "mixer").unwrap();
    assert!(line.top() < 40.0, "{line:?}");
    let roll = text(&c, dock(), "piano_roll").unwrap();
    assert!(roll.top() < 80.0, "{roll:?}");

    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "mixer")));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_some());
    assert!(text(&c, dock(), "tempo_curve").is_some());
}

#[test]
fn a_tab_group_folded_in_a_row_keeps_one_column_with_its_toggle() {
    let (mut a, c) = app(
        r#"row = [{ view = "piano_roll" }, { tabs = [{ view = "mixer" }, { view = "tempo_curve" }] }]"#,
    );
    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "mixer")));
    redraw(&mut a, &c);
    assert!(folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_none());
    assert!(text(&c, dock(), "mixer").is_none());
    assert!(text(&c, dock(), "tempo_curve").is_none());
    let right = toggles(&c, dock())
        .into_iter()
        .find(|t| t.left() > 900.0 - 40.0)
        .expect("the folded group's toggle");
    assert!(right.top() < 40.0, "{right:?}");

    frame(&mut a, &c, dock(), click(right.center()));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert!(text(&c, dock(), "Master effects").is_some());
}

#[test]
fn folding_a_tab_group_ends_its_shown_mixers_volume_gesture() {
    let (mut a, c) = app(GROUP);
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Mixer);
    let music = a.loaded.as_ref().unwrap().music.clone();
    // Held down, the button keeps a gesture going through the mixer's frame.
    let label = text(&c, dock(), "piano_roll").unwrap().center();
    frame(
        &mut a,
        &c,
        dock(),
        vec![
            egui::Event::PointerMoved(label),
            egui::Event::PointerButton {
                pos: label,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    a.undo.record_volume(music.clone(), 0, true);
    frame(&mut a, &c, dock(), vec![]);
    let pushes = a.undo.pushes();
    a.undo.record_volume(music.clone(), 0, true);
    assert_eq!(a.undo.pushes(), pushes, "the gesture went on");

    toggle(&mut a, ViewKind::TempoCurve);
    frame(&mut a, &c, dock(), vec![]);
    let pushes = a.undo.pushes();
    a.undo.record_volume(music, 0, true);
    assert_eq!(a.undo.pushes(), pushes + 1, "the folded group ended it");
}

#[test]
fn ctrl_f6_folds_the_tab_group_and_its_mixer_key_opens_it_with_the_mixer_in_front() {
    let (mut a, c) = app(HELP_FIRST);
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Help);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Mixer);
    key(&mut a, &c, dock(), Key::F6, CTRL);
    assert!(folded(&a, ViewKind::Help));
    assert!(folded(&a, ViewKind::Mixer));
    assert_eq!(focus::focused(&a), ViewKind::PianoRoll);
    redraw(&mut a, &c);

    // Behind the help tab while folded, the mixer comes out by its own key, typed into the event
    // list since the roll's keys have no M.
    let views = tabs::group(&focus::root(&a), ViewKind::Mixer).unwrap();
    tabs::select(&mut a, &views, ViewKind::Help);
    focus::set(&mut a, ViewKind::EventList);
    redraw(&mut a, &c);
    assert!(text(&c, dock(), "Master effects").is_none());
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Mixer);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(text(&c, dock(), "Master effects").is_some());
}

#[test]
fn an_opened_group_shows_the_tab_it_showed_before_folding() {
    let (mut a, c) = app(HELP_FIRST);
    let views = tabs::group(&focus::root(&a), ViewKind::Mixer).unwrap();
    tabs::select(&mut a, &views, ViewKind::Mixer);
    redraw(&mut a, &c);
    tabs::select(&mut a, &views, ViewKind::Help);
    redraw(&mut a, &c);
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Help);
    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "help")));
    redraw(&mut a, &c);
    assert!(folded(&a, ViewKind::Mixer));
    frame(&mut a, &c, dock(), click(toggle_of(&c, dock(), "help")));
    redraw(&mut a, &c);
    assert!(!folded(&a, ViewKind::Mixer));
    assert_eq!(shown(&a, ViewKind::Mixer), ViewKind::Help);
}

#[test]
fn the_palette_has_one_entry_per_tab_group_that_folds_and_opens_it() {
    let (mut a, c) = app(GROUP);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    let entries = |a: &mut App| {
        open(a, &c);
        query(a, &c, "の折りたたみ");
        let ids: Vec<String> = listed(a)
            .into_iter()
            .filter(|id| id.starts_with("pane.fold-"))
            .collect();
        press(a, &c, Key::Escape);
        ids
    };
    assert_eq!(entries(&mut a), ["pane.fold-Mixer", "pane.fold-PianoRoll"]);
    let choose = |a: &mut App, q: &str| {
        open(a, &c);
        query(a, &c, q);
        let row = listed(a)
            .iter()
            .position(|listed| listed == "pane.fold-Mixer")
            .unwrap_or_else(|| panic!("{q}: {:?}", listed(a)));
        for _ in 0..row {
            press(a, &c, Key::ArrowDown);
        }
        press(a, &c, Key::Enter);
        assert!(!a.palette.is_open());
        redraw(a, &c);
    };
    choose(&mut a, "mixer の折りたたみ");
    assert!(folded(&a, ViewKind::Mixer));
    assert!(folded(&a, ViewKind::TempoCurve));
    choose(&mut a, "mixer の折りたたみ");
    assert!(!folded(&a, ViewKind::TempoCurve));
    // The entry takes the name of the tab the group shows.
    let views = tabs::group(&focus::root(&a), ViewKind::Mixer).unwrap();
    tabs::select(&mut a, &views, ViewKind::TempoCurve);
    redraw(&mut a, &c);
    choose(&mut a, "tempo_curve の折りたたみ");
    assert!(folded(&a, ViewKind::Mixer));
}

#[test]
fn a_window_of_one_tab_group_has_no_toggle_and_no_fold_key() {
    let (mut a, c) = app(r#"tabs = [{ view = "mixer" }, { view = "tempo_curve" }]"#);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    let mixer = text(&c, dock(), "mixer").unwrap();
    assert!(toggles(&c, dock())
        .iter()
        .all(|t| (t.center().y - mixer.center().y).abs() > 6.0));
    let ids: Vec<String> =
        crate::view::commands::definitions(&a, crate::view::commands::Screen::Mixer)
            .into_iter()
            .map(|cmd| cmd.id.to_string())
            .collect();
    assert!(!ids.iter().any(|id| id.starts_with("pane.fold")), "{ids:?}");
    key(&mut a, &c, dock(), Key::F6, CTRL);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
}
