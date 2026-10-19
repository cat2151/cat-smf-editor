use super::*;
use crate::view::palette::tests::{frame, open as palette, press, query};
#[test]
fn numeric_layer_buffer_survives_and_context_confirm_cancel_preserve_target() {
    let mut a = crate::view::tests::app(cat_smf_editor::model::Music {
        format: 1,
        division: 480,
        tracks: vec![cat_smf_editor::model::Track { events: vec![] }; 2],
    });
    a.track = 1;
    open(&mut a);
    a.layer_editor.field = 1;
    a.layer_editor.begin(&a.loaded.as_ref().unwrap().patches[1]);
    let c = egui::Context::default();
    frame(&mut a, &c, vec![]);
    a.layer_editor.editing.as_mut().unwrap().text = "2".into();
    let target = a.layer_editor.selected;
    palette(&mut a, &c);
    query(&mut a, &c, "数値取消");
    press(&mut a, &c, egui::Key::Escape);
    assert_eq!(a.layer_editor.editing.as_ref().unwrap().text, "2");
    assert_eq!(a.layer_editor.selected, target);
    palette(&mut a, &c);
    query(&mut a, &c, "数値確定");
    press(&mut a, &c, egui::Key::Enter);
    assert!(a.layer_editor.editing.is_none());
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .octave,
        2
    );
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[0].layers[0]
            .settings
            .octave,
        0
    );
    a.layer_editor.begin(&a.loaded.as_ref().unwrap().patches[1]);
    frame(&mut a, &c, vec![]);
    a.layer_editor.editing.as_mut().unwrap().text = "5".into();
    palette(&mut a, &c);
    query(&mut a, &c, "数値取消");
    press(&mut a, &c, egui::Key::Enter);
    assert!(a.layer_editor.editing.is_none());
    assert_eq!(
        a.loaded.as_ref().unwrap().patches[1].layers[0]
            .settings
            .octave,
        2
    );
}
