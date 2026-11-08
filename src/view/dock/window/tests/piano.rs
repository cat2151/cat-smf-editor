//! The piano placed in a pane stays there: Ctrl+. leaves it, and its own window stays shut.
use super::*;
use crate::view::patterns::workspace::Pane;
use crate::view::recording::piano_viewport;

fn placing(views: &str) -> crate::store::layout::Spec {
    crate::store::layout::parse(&format!(
        "[root]\nview = \"event_list\"\n[[window]]\ntitle = \"Dock\"\nrect = [0, 0, 1200, 700]\ncolumn = [{views}]"
    ))
    .unwrap()
}

fn opened(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
        .is_some()
}

#[test]
fn a_placed_piano_draws_in_its_pane_ignores_ctrl_period_and_opens_no_window() {
    let (mut a, c) = every_view::showing(ViewKind::Piano);
    a.layout.spec = placing(r#"{ view = "keyboard" }, { view = "piano" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!shows(&c, dock(), "C4"));
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), "C4"));
    assert!(!opened(&c, piano_viewport()));

    key(&mut a, &c, dock(), Key::Period, Modifiers::CTRL);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(shows(&c, dock(), "C4"));
    assert!(!opened(&c, piano_viewport()));
    // A click on the piano leaves the keys with the keyboard.
    assert_eq!(focus::focused(&a), ViewKind::Keyboard);

    // Taken out of the layout, the piano opens its own window, which Ctrl+. left open.
    a.layout.spec = placing(r#"{ view = "keyboard" }"#);
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c, piano_viewport()));
}
