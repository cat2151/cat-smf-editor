//! Panes belong to the window they sit in: a menu, dialog or input covering that window
//! disables them, and the native windows retreat whichever pane has the focus.
use super::{FAMILIES, ROOT};
use crate::app::App;
use crate::view::commands::{root_owner, Screen};
use crate::view::dock::{focus, ViewKind};
use crate::view::palette::tests::windows::{fixture, ALL};
use crate::view::patterns::workspace_tests::{frame, key};
use eframe::egui::{self, Modifiers, ViewportId};

const VIEWS: [ViewKind; 4] = [
    ViewKind::EventList,
    ViewKind::Score,
    ViewKind::PianoRoll,
    ViewKind::Mixer,
];

fn place(a: &mut App, view: ViewKind) {
    a.layout.spec = crate::store::layout::parse(
        r#"
[root]
row = [
  { view = "event_list" },
  { view = "score" },
  { view = "piano_roll" },
  { view = "mixer" },
]
"#,
    )
    .unwrap();
    focus::set(a, view);
}

/// A point low in the pane, away from the dialogs centred in the window.
fn press(view: ViewKind) -> Vec<egui::Event> {
    let column = VIEWS.iter().position(|v| *v == view).unwrap() as f32;
    let pos = egui::pos2(1500.0 / 4.0 * (column + 0.5), 820.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(pos), button(true), button(false)]
}

/// Which native window families run their contents in the current state.
fn natives(a: &mut App, c: &egui::Context) -> Vec<bool> {
    FAMILIES
        .into_iter()
        .map(|native| {
            let id = ViewportId::from_hash_of(format!("pane-{native:?}"));
            let mut ran = false;
            let _ = c.run(Default::default(), |c| {
                native.show(a, c, id, Default::default(), |_, _, _| ran = true);
            });
            ran
        })
        .collect()
}

#[test]
fn a_press_moves_the_focus_between_panes_and_natives_stay_while_working() {
    for from in VIEWS {
        for to in VIEWS.into_iter().filter(|v| *v != from) {
            let (mut a, c, _) = fixture(Screen::Main).unwrap();
            place(&mut a, from);
            assert_eq!(focus::focused(&a), from);
            assert_eq!(natives(&mut a, &c), [true; 5], "{from:?}");
            frame(&mut a, &c, ROOT, vec![]);
            frame(&mut a, &c, ROOT, press(to));
            assert_eq!(focus::focused(&a), to, "{from:?} -> {to:?}");
        }
    }
}

/// `screen` with `from` focused, or `None` when the screen cannot stay open beside that focus:
/// chord timing belongs to the roll and ends once the roll gives the focus away.
fn blocked(screen: Screen, from: ViewKind) -> Option<(App, egui::Context)> {
    let (mut a, c, hosts) = fixture(screen)?;
    if hosts != [ROOT] {
        return None;
    }
    if screen == Screen::Mml {
        // The palette fixture only raises the flag; a press on the backdrop cancels the input,
        // which needs the song it was opened on.
        a.mml_input.open = false;
        crate::view::mml_input::open(&mut a, &c, false);
    }
    place(&mut a, from);
    frame(&mut a, &c, ROOT, vec![]);
    (root_owner(&a) == screen).then_some((a, c))
}

#[test]
fn menus_dialogs_and_inputs_disable_the_panes_and_hide_the_natives() {
    let mut tried = Vec::new();
    for screen in ALL.into_iter().filter(|s| !s.allows_companions()) {
        for from in VIEWS {
            let Some((mut a, c)) = blocked(screen, from) else {
                continue;
            };
            assert_eq!(natives(&mut a, &c), [false; 5], "{screen:?} / {from:?}");
            for to in VIEWS.into_iter().filter(|v| *v != from) {
                frame(&mut a, &c, ROOT, press(to));
                // The screen may end during the frame (a press on a modal's backdrop, a task
                // finishing), which says nothing about the panes beneath it.
                if root_owner(&a) != screen {
                    break;
                }
                assert_eq!(focus::focused(&a), from, "{screen:?}: {from:?} -> {to:?}");
                tried.push(screen);
            }
        }
    }
    tried.dedup();
    assert!(tried.len() >= 30, "{tried:?}");
}

#[test]
fn the_palette_keeps_the_panes_unfocused() {
    for from in VIEWS {
        for to in VIEWS.into_iter().filter(|v| *v != from) {
            let (mut a, c, _) = fixture(Screen::Main).unwrap();
            place(&mut a, from);
            frame(&mut a, &c, ROOT, vec![]);
            key(
                &mut a,
                &c,
                ROOT,
                egui::Key::P,
                Modifiers::CTRL | Modifiers::SHIFT,
            );
            assert!(a.palette.is_open(), "{from:?}");
            assert_eq!(natives(&mut a, &c), [false; 5], "{from:?}");
            frame(&mut a, &c, ROOT, press(to));
            assert_eq!(focus::focused(&a), from, "{from:?} -> {to:?}");
        }
    }
}
