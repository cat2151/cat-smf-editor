use super::*;
use crate::store::config::Store;
use crate::view::patterns::{workspace::Pane, workspace_tests as harness};

fn app() -> App {
    let mut a = crate::view::palette::tests::app();
    a.note_mode = true;
    a.patterns.workspace_open = true;
    a.patterns.workspace.scene_window = true;
    a
}

fn ids() -> [egui::ViewportId; 2] {
    [Pane::Scene.viewport(), Pane::Song.viewport()]
}

#[test]
fn native_geometry_survives_exit_reopen_and_reset_without_touching_score_or_spectrum() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("status.json");
    let c = harness::context();
    let mut a = app();
    a.config = Store::open(path.clone());
    a.config.select_analysis_window([11.0, 22.0, 700.0, 250.0]);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let rectangles = [[-1250.0, 70.0, 850.0, 470.0], [520.0, 570.0, 880.0, 500.0]];
    for (id, rect) in ids().into_iter().zip(rectangles) {
        c.data_mut(|d| d.insert_temp(egui::Id::new(("window-geometry", id)), rect));
    }
    harness::frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    for (title, rect) in TITLES.into_iter().zip(rectangles) {
        assert_eq!(a.config.pattern_windows[title], rect);
    }
    // Merely moving a native window does not write status during a drag.
    assert!(Store::open(path.clone()).pattern_windows.is_empty());
    eframe::App::on_exit(&mut a, None);
    let mut reopened = app();
    reopened.config = Store::open(path.clone());
    let fresh = harness::context();
    harness::frame(&mut reopened, &fresh, egui::ViewportId::ROOT, vec![]);
    for (id, [x, y, w, h]) in ids().into_iter().zip(rectangles) {
        let b = fresh
            .data(|d| d.get_temp::<ViewportBuilder>(egui::Id::new(("window-builder", id))))
            .unwrap();
        assert_eq!(b.position, Some(egui::pos2(x, y)));
        assert_eq!(b.inner_size, Some(egui::vec2(w, h)));
    }
    // Real palette invocation works from a child and leaves the score intact.
    harness::command(
        &mut reopened,
        &fresh,
        Pane::Song.viewport(),
        "reset pattern windows position",
    );
    assert!(reopened.config.pattern_windows.is_empty());
    assert!(Store::open(path.clone()).pattern_windows.is_empty());
    assert_eq!(
        Store::open(path).analysis_window,
        Some([11.0, 22.0, 700.0, 250.0])
    );
    assert_eq!(
        cat_smf_editor::smf_io::encode(&reopened.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    // A reset frame's old native rectangles cannot overwrite the reset.
    let _ = fresh.run(Default::default(), |ctx| {
        reset(&mut reopened, ctx);
    });
    for (id, rect) in ids().into_iter().zip(rectangles) {
        fresh.data_mut(|d| d.insert_temp(egui::Id::new(("window-geometry", id)), rect));
    }
    reopened.comma_menu.open = false;
    harness::frame(&mut reopened, &fresh, egui::ViewportId::ROOT, vec![]);
    assert!(reopened.config.pattern_windows.is_empty());
}

#[test]
fn old_status_retains_settings_and_rejects_invalid_window_sizes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("status.json");
    std::fs::write(&path, r#"{"session":null,"analysis_window":[-10,20,700,250],"analysis_open":true,"meta_text_visible":false}"#).unwrap();
    let mut store = Store::open(path.clone());
    assert!(store.error.is_none());
    assert!(store.pattern_windows.is_empty());
    store
        .pattern_windows
        .insert("Scene".into(), [10.0, 20.0, 0.0, 500.0]);
    store
        .pattern_windows
        .insert("Song".into(), [-10.0, 20.0, 800.0, 500.0]);
    store.persist();
    let reopened = Store::open(path);
    assert!(!reopened.pattern_windows.contains_key("Scene"));
    assert!(reopened.pattern_windows.contains_key("Song"));
    assert!(reopened.analysis_open);
    assert!(!reopened.meta_text_visible);
    assert_eq!(reopened.analysis_window, Some([-10.0, 20.0, 700.0, 250.0]));
}
