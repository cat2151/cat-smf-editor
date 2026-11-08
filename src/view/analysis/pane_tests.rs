//! The analysis placed in a pane draws there instead of in a window of its own.
use super::*;
use crate::view::commands::{owner, Screen};
use crate::view::dock::{focus, window, ViewKind};
use crate::view::patterns::workspace_tests::{context, frame, key};
use eframe::egui::{Key, Modifiers};

const ROOT: egui::ViewportId = egui::ViewportId::ROOT;
/// The background of the spectrum, the Lissajous and the correlation bar.
const PLOT: egui::Color32 = egui::Color32::from_rgb(15, 20, 24);

fn app() -> App {
    let mut a = crate::view::palette::tests::app();
    a.layout.spec = crate::store::layout::parse(
        r#"
[[window]]
title = "Dock"
rect = [0, 0, 900, 400]
view = "analysis"
"#,
    )
    .unwrap();
    a
}

fn dock() -> egui::ViewportId {
    window::viewport(0)
}

fn plots(c: &egui::Context, id: egui::ViewportId) -> usize {
    c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new(("workspace-shapes", id)))
    })
    .unwrap_or_default()
    .iter()
    .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == PLOT))
    .count()
}

fn opened(c: &egui::Context, id: egui::ViewportId) -> bool {
    c.data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
        .is_some()
}

#[test]
fn the_placed_analysis_draws_in_its_pane_and_opens_no_window() {
    let mut a = app();
    let c = context();
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(plots(&c, dock()), 0);
    toggle(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(plots(&c, dock()), 3);
    assert!(!opened(&c, a.analysis.viewport()));
    // The pane keeps the frames coming.
    assert!(c.has_requested_repaint_for(&ROOT));
    toggle(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    assert_eq!(plots(&c, dock()), 0);
    assert!(!opened(&c, a.analysis.viewport()));
}

#[test]
fn a_key_in_its_dock_window_reaches_the_analysis_and_closing_it_returns_the_keys() {
    let mut a = app();
    let c = context();
    toggle(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    key(&mut a, &c, dock(), Key::F6, Modifiers::NONE);
    assert_eq!(focus::focused(&a), ViewKind::Analysis);
    // As with every pane, the main window's keys follow the focus.
    assert_eq!(owner(&a, &c), Screen::Analysis);
    key(&mut a, &c, dock(), Key::Escape, Modifiers::NONE);
    assert!(!a.analysis.is_open());
    // The empty pane gives up the focus, so the main window's keys work again.
    assert_eq!(focus::focused(&a), ViewKind::EventList);
}

#[test]
fn taking_the_analysis_out_of_the_layout_brings_its_window_back() {
    let mut a = app();
    let c = context();
    toggle(&mut a);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(!opened(&c, a.analysis.viewport()));
    a.layout.spec = crate::store::layout::parse("").unwrap();
    frame(&mut a, &c, ROOT, vec![]);
    frame(&mut a, &c, ROOT, vec![]);
    assert!(opened(&c, a.analysis.viewport()));
    assert_eq!(plots(&c, a.analysis.viewport()), 3);
}
