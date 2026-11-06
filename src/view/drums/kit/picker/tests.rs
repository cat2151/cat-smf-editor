use super::super::tests::{frame, key};
use super::*;

fn app(rows: Vec<Option<u8>>) -> App {
    let mut a = super::super::super::kit_edit_tests::fixture(128);
    let mut patch = a.loaded.as_ref().unwrap().patches[1].clone();
    patch.kit.as_mut().unwrap().rows = Some(rows);
    crate::player::selection::store_track(&mut a, 1, &patch).unwrap();
    super::super::super::open(&mut a);
    super::super::sync(&mut a);
    a
}

fn rect(ctx: &egui::Context, note: u8) -> egui::Rect {
    ctx.data(|d| {
        d.get_temp(egui::Id::new(("kit-picker-note", note)))
            .unwrap()
    })
}

#[test]
fn unassigned_row_opens_at_36_centered_and_lists_notes_descending() {
    let mut a = app(vec![None]);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let ctx = egui::Context::default();
    key(&mut a, &ctx, Key::N, egui::Modifiers::NONE);
    for _ in 0..4 {
        frame(&mut a, &ctx, vec![], 1200.0);
    }
    assert_eq!(
        a.drum_editor.kit.picker.as_ref().unwrap().selected,
        Some(36)
    );
    let clip: egui::Rect = ctx.data(|d| d.get_temp(egui::Id::new("kit-picker-clip")).unwrap());
    assert!(clip.contains_rect(rect(&ctx, 36)));
    assert!(
        (rect(&ctx, 36).center().y - clip.center().y).abs() < 2.0,
        "note {:?}, clip {clip:?}",
        rect(&ctx, 36)
    );
    assert!(rect(&ctx, 37).top() < rect(&ctx, 36).top());
    assert!(rect(&ctx, 36).top() < rect(&ctx, 35).top());
    key(&mut a, &ctx, Key::ArrowDown, egui::Modifiers::NONE);
    assert_eq!(
        a.drum_editor.kit.picker.as_ref().unwrap().selected,
        Some(35)
    );
    key(&mut a, &ctx, Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(
        a.drum_editor.kit.picker.as_ref().unwrap().selected,
        Some(36)
    );
    key(&mut a, &ctx, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(
        before,
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
    );
}

#[test]
fn opening_preserves_current_note_or_chooses_nearest_available_to_36() {
    for (rows, expected) in [(vec![Some(90)], Some(90)), (vec![None, Some(36)], Some(37))] {
        let mut a = app(rows);
        open(&mut a).unwrap();
        assert_eq!(
            a.drum_editor.kit.picker.as_ref().unwrap().selected,
            expected
        );
        assert!(candidates(&a)
            .windows(2)
            .all(|pair| pair[0].note > pair[1].note));
    }
    let mut a = app(vec![Some(90)]);
    a.loaded.as_mut().unwrap().patches[1]
        .kit
        .as_mut()
        .unwrap()
        .notes
        .retain(|n| n.note != 90);
    open(&mut a).unwrap();
    assert_eq!(
        a.drum_editor.kit.picker.as_ref().unwrap().selected,
        Some(36)
    );
}
