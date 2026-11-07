use super::*;

#[test]
fn the_keyboard_window_opens_clear_of_the_song_window() {
    let (mut a, c, _, _, _) = verification_song();
    // The default Pattern / Song window, which covers the middle of the editor.
    let song = [460.0, 520.0, 760.0, 520.0];
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("window-geometry", Pane::Song.viewport())),
            song,
        )
    });
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    let keyboard = crate::view::recording::window::viewport();
    let builder = |c: &egui::Context| {
        c.data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", keyboard))))
            .unwrap()
    };
    key(&mut a, &c, egui::ViewportId::ROOT, Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    let b = builder(&c);
    let opened = egui::Rect::from_min_size(
        b.position.unwrap(),
        b.inner_size.unwrap() + egui::vec2(16.0, 40.0),
    );
    let [x, y, w, h] = song;
    let song = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w + 16.0, h + 40.0));
    assert!(!opened.intersects(song), "{opened:?} {song:?}");
}

#[test]
fn keyboard_windows_reopen_where_moved_after_exit_until_reset() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("status.json");
    let keyboard = crate::view::recording::window::viewport();
    let piano = crate::view::recording::piano_viewport();
    let (mut a, c, _, _, _) = verification_song();
    a.config = crate::store::config::Store::open(path.clone());
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    for (id, rect) in [
        (keyboard, [-900.0, 30.0, 713.0, 284.0]),
        (piano, [300.0, 700.0, 600.0, 120.0]),
    ] {
        c.data_mut(|d| d.insert_temp::<[f32; 4]>(egui::Id::new(("window-geometry", id)), rect));
    }
    frame(&mut a, &c, keyboard, vec![]);
    assert_eq!(a.config.keyboard_windows["Keyboard"], [-900.0, 30.0]);
    assert_eq!(a.config.keyboard_windows["Piano"], [300.0, 700.0]);
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
    eframe::App::on_exit(&mut a, None);

    let (mut reopened, fresh, _, _, _) = verification_song();
    reopened.config = crate::store::config::Store::open(path.clone());
    let position = |c: &egui::Context, id| {
        c.data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
            .unwrap()
            .position
    };
    key(
        &mut reopened,
        &fresh,
        Pane::Song.viewport(),
        Key::R,
        Modifiers::SHIFT,
    );
    frame(&mut reopened, &fresh, keyboard, vec![]);
    assert_eq!(position(&fresh, keyboard), Some(egui::pos2(-900.0, 30.0)));
    assert_eq!(position(&fresh, piano), Some(egui::pos2(300.0, 700.0)));
    key(
        &mut reopened,
        &fresh,
        keyboard,
        Key::Escape,
        Modifiers::NONE,
    );

    command(
        &mut reopened,
        &fresh,
        Pane::Song.viewport(),
        "keyboard.reset-windows",
    );
    assert!(reopened.config.keyboard_windows.is_empty());
    assert!(crate::store::config::Store::open(path)
        .keyboard_windows
        .is_empty());
}

#[test]
fn a_resumed_phrase_loop_opens_the_keyboard_once_the_song_window_has_its_place() {
    let (mut a, c, _, _, _) = verification_song();
    a.config.phrase_loop = true;
    crate::view::recording::resume(&mut a);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(!a.patterns.recording.is_open());
    let song = [460.0, 520.0, 760.0, 520.0];
    c.data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("window-geometry", Pane::Song.viewport())),
            song,
        )
    });
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    let keyboard = crate::view::recording::window::viewport();
    let b = c
        .data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", keyboard))))
        .unwrap();
    let opened = egui::Rect::from_min_size(
        b.position.unwrap(),
        b.inner_size.unwrap() + egui::vec2(16.0, 40.0),
    );
    let [x, y, w, h] = song;
    let song = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w + 16.0, h + 40.0));
    assert!(!opened.intersects(song), "{opened:?} {song:?}");
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
}

#[test]
fn keyboard_windows_hide_while_the_editor_asks_and_return_after() {
    use crate::view::recording::{capture_viewport, piano_viewport, settings_viewport};
    let (mut a, c, _, _, _) = verification_song();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    let keyboard = crate::view::recording::window::viewport();
    key(&mut a, &c, keyboard, Key::Comma, Modifiers::CTRL);
    key(&mut a, &c, keyboard, Key::J, Modifiers::CTRL);
    let windows = [
        keyboard,
        settings_viewport(),
        piano_viewport(),
        capture_viewport(),
    ];
    let shown = |c: &egui::Context| {
        windows.map(|id| {
            c.data(|d| d.get_temp::<egui::ViewportBuilder>(egui::Id::new(("window-builder", id))))
                .unwrap_or_else(|| panic!("{id:?} not opened"))
                .visible
                .unwrap_or(true)
        })
    };
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert_eq!(shown(&c), [true; 4]);
    // Owned windows sit above the editor, so its confirmation would be under them.
    a.toggle_auto_save();
    assert!(a.config.navigation.departure.is_some());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert_eq!(shown(&c), [false; 4]);
    key(
        &mut a,
        &c,
        egui::ViewportId::ROOT,
        Key::Escape,
        Modifiers::NONE,
    );
    assert!(a.config.navigation.departure.is_none());
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert_eq!(shown(&c), [true; 4]);
    assert!(a.patterns.recording.is_open());
}
