use super::*;
use crate::view::palette::tests::{frame, open, press, query};
#[test]
fn note_buffer_and_selection_survive_palette_and_context_actions_apply_once() {
    let mut a = crate::view::tests::app(super::tests::chord_song());
    a.track = 1;
    let c = egui::Context::default();
    frame(&mut a, &c, vec![]);
    begin(
        &mut a,
        Field::Duration,
        1,
        0,
        egui::Rect::from_min_size(egui::pos2(600.0, 300.0), egui::vec2(90.0, 24.0)),
        "960",
    );
    frame(&mut a, &c, vec![]);
    a.note_edit.as_mut().unwrap().text = "480".into();
    let focus = c.memory(|m| m.focused());
    open(&mut a, &c);
    query(&mut a, &c, "取消");
    press(&mut a, &c, egui::Key::Escape);
    assert_eq!(a.note_edit.as_ref().unwrap().text, "480");
    assert_eq!(c.memory(|m| m.focused()), focus);
    assert!(!a.undo.available());
    open(&mut a, &c);
    query(&mut a, &c, "確定");
    press(&mut a, &c, egui::Key::Enter);
    assert!(a.note_edit.is_none());
    assert!(a.undo.available());
    assert_eq!(
        a.loaded.as_ref().unwrap().note_views[1].durations[0],
        Some(480)
    );
    begin(
        &mut a,
        Field::Duration,
        1,
        0,
        egui::Rect::from_min_size(egui::pos2(600.0, 300.0), egui::vec2(90.0, 24.0)),
        "480",
    );
    frame(&mut a, &c, vec![]);
    open(&mut a, &c);
    query(&mut a, &c, "取消");
    press(&mut a, &c, egui::Key::Enter);
    assert!(a.note_edit.is_none());
}
