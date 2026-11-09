//! Tabs: the header switching them, the focus going with the shown one, closed tabs giving way
//! to the first, hidden ones stopping, and the keys moving between them.
use super::*;
use crate::view::commands::{self, Screen};
use crate::view::dock::window::viewport;
use crate::view::patterns::workspace_tests::{frame, key, verification_song};
use eframe::egui::{Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);

fn dock() -> egui::ViewportId {
    viewport(0)
}

/// A song with `root` as the main window's tree and `window` in a 900×700 dock window.
fn app(root: &str, window: &str) -> (App, egui::Context) {
    let (mut a, c, ..) = verification_song();
    a.layout.spec = crate::store::layout::parse(&format!(
        "[root]\n{root}\n[[window]]\ntitle = \"Dock\"\nrect = [0, 0, 900, 700]\n{window}"
    ))
    .unwrap();
    redraw(&mut a, &c);
    (a, c)
}

fn redraw(a: &mut App, c: &egui::Context) {
    frame(a, c, ROOT, vec![]);
    frame(a, c, ROOT, vec![]);
}

fn shapes(c: &egui::Context, id: egui::ViewportId) -> Vec<egui::epaint::ClippedShape> {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
}

fn text(c: &egui::Context, id: egui::ViewportId, text: &str) -> Option<egui::Rect> {
    shapes(c, id).into_iter().find_map(|s| match &s.shape {
        egui::Shape::Text(t) if t.galley.text() == text => Some(t.visual_bounding_rect()),
        _ => None,
    })
}

fn click(a: &mut App, c: &egui::Context, id: egui::ViewportId, name: &str) {
    let pos = text(c, id, name)
        .unwrap_or_else(|| panic!("no {name}"))
        .center();
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame(
        a,
        c,
        id,
        vec![egui::Event::PointerMoved(pos), button(true), button(false)],
    );
    redraw(a, c);
}

fn group_of(a: &App, view: ViewKind) -> Vec<ViewKind> {
    group(&focus::root(a), view).unwrap()
}

#[test]
fn a_click_on_a_tab_shows_it_and_gives_it_the_focus() {
    let (mut a, c) = app(
        r#"view = "event_list""#,
        r#"tabs = [{ view = "mixer" }, { view = "tempo_curve" }]"#,
    );
    let views = group_of(&a, ViewKind::Mixer);
    assert_eq!(shown(&a, &views), ViewKind::Mixer);
    assert!(text(&c, dock(), "Master effects").is_some());
    assert!(text(&c, dock(), "tempo_curve").is_some());

    click(&mut a, &c, dock(), "tempo_curve");
    assert_eq!(shown(&a, &views), ViewKind::TempoCurve);
    assert_eq!(focus::focused(&a), ViewKind::TempoCurve);
    assert!(a.tempo_curve.open);
    assert!(text(&c, dock(), "Master effects").is_none());

    click(&mut a, &c, dock(), "mixer");
    assert_eq!(shown(&a, &views), ViewKind::Mixer);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!a.tempo_curve.open);
    assert!(text(&c, dock(), "Master effects").is_some());
}

#[test]
fn the_track_overview_tab_opens_the_overview() {
    let (mut a, c) = app(
        r#"row = [{ view = "event_list" }, { tabs = [{ view = "score" }, { view = "track_overview" }] }]"#,
        r#"view = "mixer""#,
    );
    assert!(!a.track_overview.open);
    click(&mut a, &c, ROOT, "track_overview");
    assert!(a.track_overview.open);
    assert_eq!(focus::focused(&a), ViewKind::TrackOverview);
    assert_eq!(
        shown(&a, &group_of(&a, ViewKind::Score)),
        ViewKind::TrackOverview
    );
}

#[test]
fn a_closed_tab_gives_way_to_the_first_one() {
    let (mut a, c) = app(
        r#"tabs = [{ view = "event_list" }, { view = "piano_roll" }]"#,
        r#"view = "mixer""#,
    );
    let views = group_of(&a, ViewKind::EventList);
    click(&mut a, &c, ROOT, "piano_roll");
    assert!(a.piano_roll.open);
    assert_eq!(shown(&a, &views), ViewKind::PianoRoll);
    assert!(hidden(&a, ViewKind::EventList));

    // The focus moving to the other window closes the roll, and the first tab comes back.
    let pos = text(&c, dock(), "Master effects").unwrap().center();
    frame(
        &mut a,
        &c,
        dock(),
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert!(!a.piano_roll.open);
    assert_eq!(shown(&a, &views), ViewKind::EventList);
    redraw(&mut a, &c);
    assert_eq!(a.pane_tabs.stored(&views), ViewKind::EventList);
}

#[test]
fn a_hidden_tab_is_not_drawn_and_ends_its_gesture() {
    let (mut a, c) = app(
        r#"view = "event_list""#,
        r#"tabs = [{ view = "mixer" }, { view = "help" }]"#,
    );
    let music = a.loaded.as_ref().unwrap().music.clone();
    // Held down, the button keeps a gesture going through the mixer's frame.
    let pos = text(&c, dock(), "mixer").unwrap().center();
    frame(
        &mut a,
        &c,
        dock(),
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
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

    // Help takes no focus, so only the tab changes.
    let views = group_of(&a, ViewKind::Mixer);
    select(&mut a, &views, ViewKind::Help);
    frame(&mut a, &c, dock(), vec![]);
    assert!(text(&c, dock(), "Master effects").is_none());
    let pushes = a.undo.pushes();
    a.undo.record_volume(music, 0, true);
    assert_eq!(a.undo.pushes(), pushes + 1, "the hidden mixer ended it");
}

#[test]
fn f6_passes_over_hidden_tabs_and_ctrl_page_keys_go_round_them() {
    let (mut a, c) = app(
        r#"row = [{ view = "event_list" }, { tabs = [{ view = "mixer" }, { view = "piano_roll" }, { view = "tempo_curve" }] }]"#,
        r#"view = "help""#,
    );
    let views = group_of(&a, ViewKind::Mixer);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    // Only the shown tab counts as a pane.
    assert_eq!(focus::panes(&a), 2);
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    key(&mut a, &c, ROOT, Key::F6, Modifiers::SHIFT);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);

    for expected in [
        ViewKind::PianoRoll,
        ViewKind::TempoCurve,
        ViewKind::Mixer,
        ViewKind::PianoRoll,
    ] {
        key(&mut a, &c, ROOT, Key::PageDown, CTRL);
        assert_eq!(focus::focused(&a), expected);
        assert_eq!(shown(&a, &views), expected);
    }
    for expected in [ViewKind::Mixer, ViewKind::TempoCurve] {
        key(&mut a, &c, ROOT, Key::PageUp, CTRL);
        assert_eq!(focus::focused(&a), expected);
        assert_eq!(shown(&a, &views), expected);
    }
    assert!(a.tempo_curve.open && !a.piano_roll.open);
    // F6 from the tempo curve goes to the event list, not to the hidden mixer or roll.
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
    // The event list is no tab, so the page keys do nothing there.
    key(&mut a, &c, ROOT, Key::PageDown, CTRL);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
}

#[test]
fn a_view_opened_by_its_own_key_comes_to_the_front() {
    let (mut a, c) = app(
        r#"row = [{ view = "event_list" }, { tabs = [{ view = "score" }, { view = "mixer" }] }]"#,
        r#"view = "help""#,
    );
    let views = group_of(&a, ViewKind::Score);
    assert_eq!(shown(&a, &views), ViewKind::Score);
    key(&mut a, &c, ROOT, Key::M, Modifiers::NONE);
    redraw(&mut a, &c);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    assert_eq!(shown(&a, &views), ViewKind::Mixer);
    assert!(text(&c, ROOT, "Master effects").is_some());
}

#[test]
fn ctrl_f6_folds_the_whole_tab_group() {
    let (mut a, c) = app(
        r#"row = [{ view = "event_list" }, { tabs = [{ view = "mixer" }, { view = "tempo_curve" }] }]"#,
        r#"view = "help""#,
    );
    key(&mut a, &c, ROOT, Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Mixer);
    key(&mut a, &c, ROOT, Key::F6, CTRL);
    assert!(crate::view::dock::collapse::folded(&a, ViewKind::Mixer));
    assert!(crate::view::dock::collapse::folded(
        &a,
        ViewKind::TempoCurve
    ));
    assert_eq!(focus::focused(&a), ViewKind::EventList);
}

/// Every screen named in the palette definitions, and the global one.
fn screens() -> Vec<Screen> {
    let text = include_str!("../../../../config/command-palette.toml");
    let table: toml::Table = toml::from_str(text).unwrap();
    let mut screens = vec![Screen::Global];
    for name in table["screens"].as_table().unwrap().keys() {
        screens.push(toml::Value::String(name.clone()).try_into().unwrap());
    }
    screens
}

/// A command bound to a page key, with its screen.
type Bound = (Screen, String, Vec<(Key, Modifiers)>);

fn page_keys(a: &App) -> Vec<Bound> {
    let mut out = Vec::new();
    for screen in screens() {
        for cmd in commands::definitions(a, screen) {
            if cmd
                .keys
                .iter()
                .any(|(k, _)| matches!(k, Key::PageUp | Key::PageDown))
            {
                out.push((screen, cmd.id, cmd.keys));
            }
        }
    }
    out
}

#[test]
fn ctrl_page_keys_take_nothing_from_the_plain_page_keys() {
    let (mut a, c) = app(r#"view = "event_list""#, r#"view = "help""#);
    let before = page_keys(&a);
    assert!(
        before
            .iter()
            .any(|(s, id, _)| *s == Screen::MacroEditor && id == "macro.variation-prev"),
        "{before:?}"
    );
    a.layout.spec = crate::store::layout::parse(
        "[root]\ntabs = [{ view = \"event_list\" }, { view = \"score\" }]",
    )
    .unwrap();
    redraw(&mut a, &c);
    assert!(switchable(&a));
    let after = page_keys(&a);
    let ctrl = |keys: &[(Key, Modifiers)]| keys.iter().any(|(_, m)| m.ctrl);
    for (screen, id, keys) in &after {
        if ctrl(keys) {
            assert!(id.starts_with("pane.tab-"), "{screen:?} {id}");
        }
    }
    let plain = |list: &[Bound]| {
        list.iter()
            .filter(|(_, _, keys)| !ctrl(keys))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(plain(&before), plain(&after));
    // A plain PageDown still moves the event list's cursor.
    let row = a.cursor_row;
    key(&mut a, &c, ROOT, Key::PageDown, Modifiers::NONE);
    assert_ne!(a.cursor_row, row);
    assert_eq!(focus::focused(&a), ViewKind::EventList);
}
