use super::*;
use std::time::Duration;

/// Press and release a recording pad at its physical position.
fn pad(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key: k,
            physical_key: Some(k),
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .collect();
    frame(a, c, id, events);
}

#[test]
fn song_and_scene_record_into_new_unplaced_patterns_and_keep_the_origin_window() {
    let (mut a, c, first, _, _) = verification_song();
    let original_body = body(&a, first);
    for pane in [Pane::Song, Pane::Scene] {
        let before = Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns
            .len();
        let arrangement = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
        let song_cells = arrangement.song.len();
        let scene_cells = arrangement
            .scenes
            .iter()
            .map(|s| s.cells.len())
            .sum::<usize>();
        let track = a.track;
        key(&mut a, &c, pane.viewport(), Key::R, Modifiers::SHIFT);
        assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
        // The pads have their own window; keys of the initiating window do not play them.
        let keyboard = crate::view::recording::window::viewport();
        pad(&mut a, &c, pane.viewport(), Key::Z);
        pad(&mut a, &c, keyboard, Key::Space);
        pad(&mut a, &c, keyboard, Key::Z);
        pad(&mut a, &c, keyboard, Key::C);
        key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
        assert!(!a.patterns.recording.is_open());
        assert!(mode::workspace_open(&a));
        assert_eq!(a.track, track);
        let id = a.patterns.selected.unwrap();
        assert_eq!(
            a.patterns.workspace.selection(pane).choice,
            Some(PatternOrScene::Pattern(id))
        );
        assert_eq!(
            Catalog::read(&a.loaded.as_ref().unwrap().music)
                .unwrap()
                .patterns
                .len(),
            before + 1
        );
        assert_eq!(body(&a, id), vec![60, 64]);
        assert_eq!(body(&a, first), original_body);
        let arrangement = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
        assert_eq!(arrangement.song.len(), song_cells);
        assert_eq!(
            arrangement
                .scenes
                .iter()
                .map(|s| s.cells.len())
                .sum::<usize>(),
            scene_cells
        );
        assert!(!a.playback.open && a.playback.active.is_none());
    }
}

#[test]
fn song_loop_laps_become_candidates_and_the_last_take_is_chosen() {
    let (mut a, c, _, _, _) = verification_song();
    let id = crate::view::recording::window::viewport();
    let before = choices(&a, Pane::Song).len();
    let existing = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .into_keys()
        .collect::<Vec<_>>();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(!a.patterns.recording.preparing());
    key(&mut a, &c, id, Key::Comma, Modifiers::CTRL);
    // Loop, 1 measure, BPM 90 in the settings window beside the pads.
    let settings = crate::view::recording::settings_viewport();
    key(&mut a, &c, settings, Key::ArrowRight, Modifiers::NONE);
    key(&mut a, &c, settings, Key::ArrowDown, Modifiers::NONE);
    key(&mut a, &c, settings, Key::ArrowDown, Modifiers::NONE);
    for _ in 0..3 {
        key(&mut a, &c, settings, Key::ArrowLeft, Modifiers::NONE);
    }
    key(&mut a, &c, settings, Key::ArrowDown, Modifiers::NONE);
    for _ in 0..3 {
        key(&mut a, &c, settings, Key::ArrowLeft, Modifiers::SHIFT);
    }
    key(&mut a, &c, settings, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open() && !a.patterns.recording.preparing());
    let measure = Duration::from_millis(2667);
    key(&mut a, &c, id, Key::Space, Modifiers::NONE);
    crate::view::recording::render_click(&mut a);
    crate::view::recording::rewind(&mut a, measure + Duration::from_millis(100));
    pad(&mut a, &c, id, Key::Z);
    crate::view::recording::rewind(&mut a, measure);
    pad(&mut a, &c, id, Key::C);
    key(&mut a, &c, id, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.recording.is_open(), "{:?}", a.load_error);
    let after = choices(&a, Pane::Song);
    assert_eq!(after.len(), before + 2);
    let last = a.patterns.selected.unwrap();
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(last))
    );
    assert_eq!(body(&a, last), vec![64]);
    let c = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let created = c
        .patterns
        .keys()
        .copied()
        .filter(|p| !existing.contains(p))
        .collect::<Vec<_>>();
    assert_eq!(created.len(), 2);
    let first = created.into_iter().find(|&p| p != last).unwrap();
    assert_eq!(body(&a, first), vec![60]);
    assert!([first, last]
        .iter()
        .all(|p| c.patterns[p].meters.len() == 1));
    assert!(
        (a.patterns.tempo.0 - 90.0).abs() < 1e-3,
        "{}",
        a.patterns.tempo.0
    );
}

#[test]
fn the_keyboard_window_opens_clear_of_the_song_window_and_reopens_where_it_was_left() {
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
    // A window the user moved reopens there.
    let moved: [f32; 4] = [40.0, 30.0, 700.0, 300.0];
    c.data_mut(|d| d.insert_temp(egui::Id::new(("window-geometry", keyboard)), moved));
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.recording.is_open());
    key(&mut a, &c, egui::ViewportId::ROOT, Key::R, Modifiers::SHIFT);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert_eq!(builder(&c).position, Some(egui::pos2(40.0, 30.0)));
}

#[test]
fn the_palette_finds_the_keyboard_by_recording_words() {
    let (a, _, _, _, _) = verification_song();
    for screen in [Screen::Main, Screen::Song] {
        let defs = crate::view::commands::definitions(&a, screen);
        let mut q = crate::view::commands::SearchQuery::default();
        for text in ["rec", "recording", "録音"] {
            q.update(text);
            assert!(
                defs.iter()
                    .any(|c| c.id == "record-pattern" && c.matches_query(&q)),
                "{screen:?} {text}"
            );
        }
    }
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
    assert!(!a.patterns.recording.is_open());
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
        "reset virtual midi keyboard windows position",
    );
    assert!(reopened.config.keyboard_windows.is_empty());
    assert!(crate::store::config::Store::open(path)
        .keyboard_windows
        .is_empty());
    reopened.comma_menu.open = false;
    key(
        &mut reopened,
        &fresh,
        Pane::Song.viewport(),
        Key::R,
        Modifiers::SHIFT,
    );
    assert!(reopened.patterns.recording.is_open());
    frame(&mut reopened, &fresh, keyboard, vec![]);
    assert_ne!(position(&fresh, keyboard), Some(egui::pos2(-900.0, 30.0)));
    assert_ne!(position(&fresh, piano), Some(egui::pos2(300.0, 700.0)));
}

#[test]
fn the_editor_windows_stay_usable_while_the_keyboard_is_open() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let keyboard = crate::view::recording::window::viewport();
    let before = bytes(&a);
    focus(&c, song);
    key(&mut a, &c, song, Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    // Help follows the focused window, not the open keyboard.
    frame(&mut a, &c, song, vec![]);
    assert_eq!(crate::view::help::test_target(&a), "Song");
    focus(&c, keyboard);
    frame(&mut a, &c, keyboard, vec![]);
    assert_eq!(crate::view::help::test_target(&a), "Recording");
    focus(&c, song);
    frame(&mut a, &c, song, vec![]);
    assert_eq!(crate::view::help::test_target(&a), "Song");
    frame(&mut a, &c, song, vec![egui::Event::Text("?".into())]);
    assert!(a.help_open && a.patterns.recording.is_open());
    // Song keys reach the Song window; the keyboard stays open and silent.
    let scene = a.patterns.workspace.scene_window;
    key(&mut a, &c, song, Key::S, Modifiers::NONE);
    assert_ne!(a.patterns.workspace.scene_window, scene);
    assert!(a.patterns.recording.is_open());
    assert!(!(0..128).any(|p| crate::view::recording::sounding(&a, p)));
    key(&mut a, &c, song, Key::P, Modifiers::CTRL | Modifiers::SHIFT);
    assert!(a.palette.is_open());
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    assert!(!a.palette.is_open() && a.patterns.recording.is_open());
    // Shift+R again brings the open keyboard forward instead of a second one.
    c.data_mut(|d| d.remove::<bool>(egui::Id::new(("workspace-focus", keyboard))));
    key(&mut a, &c, song, Key::R, Modifiers::SHIFT);
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-focus", keyboard))))
        .unwrap_or(false));
    // The editor edits while the keyboard is open.
    focus(&c, egui::ViewportId::ROOT);
    key(&mut a, &c, egui::ViewportId::ROOT, Key::I, Modifiers::NONE);
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.mml_input.open && a.patterns.recording.is_open());
    key(
        &mut a,
        &c,
        egui::ViewportId::ROOT,
        Key::Escape,
        Modifiers::NONE,
    );
    // A discarded take leaves the song as it was.
    focus(&c, keyboard);
    pad(&mut a, &c, keyboard, Key::Space);
    pad(&mut a, &c, keyboard, Key::Z);
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::SHIFT);
    assert!(!a.patterns.recording.is_open());
    assert_eq!(bytes(&a), before);
}
