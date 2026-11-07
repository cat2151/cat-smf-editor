use super::*;
use std::time::Duration;
mod windows;

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
/// Patterns other than the keyboard's Capture.
fn normal(a: &App) -> Vec<u64> {
    Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .values()
        .filter(|p| p.kind == cat_smf_editor::score::pattern::Kind::Normal)
        .map(|p| p.id)
        .collect()
}

#[test]
fn song_and_scene_record_into_new_unplaced_patterns_and_keep_the_origin_window() {
    let (mut a, c, first, _, _) = verification_song();
    let original_body = body(&a, first);
    for pane in [Pane::Song, Pane::Scene] {
        let before = normal(&a).len();
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
        assert!(a.patterns.recording.is_open());
        assert!(mode::workspace_open(&a));
        assert_eq!(a.track, track);
        let id = a.patterns.selected.unwrap();
        assert_eq!(
            a.patterns.workspace.selection(pane).choice,
            Some(PatternOrScene::Pattern(id))
        );
        assert_eq!(normal(&a).len(), before + 1);
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
    let existing = normal(&a);
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    assert!(!a.patterns.recording.preparing());
    key(&mut a, &c, id, Key::Comma, Modifiers::CTRL);
    // Loop, 1 measure, BPM 90 in the settings window beside the pads.
    let settings = crate::view::recording::settings_viewport();
    key(&mut a, &c, settings, Key::ArrowRight, Modifiers::NONE);
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
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    let after = choices(&a, Pane::Song);
    // Two laps; the Capture of their keys is not a candidate.
    assert_eq!(after.len(), before + 2);
    let last = a.patterns.selected.unwrap();
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(last))
    );
    assert_eq!(body(&a, last), vec![64]);
    let created = normal(&a)
        .into_iter()
        .filter(|p| !existing.contains(p))
        .collect::<Vec<_>>();
    assert_eq!(created.len(), 2);
    let first = created.into_iter().find(|&p| p != last).unwrap();
    assert_eq!(body(&a, first), vec![60]);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert!([first, last]
        .iter()
        .all(|p| catalog.patterns[p].meters.len() == 1));
    // Recording at 90 BPM must not override the conductor used for later auditions.
    assert_eq!(super::super::timing::bpm(&a), 120.0);
}

#[test]
fn keys_played_into_the_capture_leave_it_out_of_the_song_and_scene_candidates() {
    let (mut a, c, _, _, _) = verification_song();
    let keyboard = crate::view::recording::window::viewport();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open(), "{:?}", a.load_error);
    pad(&mut a, &c, keyboard, Key::Z);
    pad(&mut a, &c, keyboard, Key::C);
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    assert!(a.patterns.recording.is_open());
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(music).unwrap();
    let capture = catalog
        .patterns
        .values()
        .filter(|p| p.kind == cat_smf_editor::score::pattern::Kind::Capture)
        .map(|p| PatternOrScene::Pattern(p.id))
        .collect::<Vec<_>>();
    assert_eq!(capture.len(), 1);
    let mut patterns = normal(&a);
    patterns.sort();
    let scenes = scene::read(music)
        .unwrap()
        .scenes
        .iter()
        .map(|s| PatternOrScene::Scene(s.id))
        .collect::<Vec<_>>();
    assert!(!scenes.is_empty());
    for (pane, offered_scenes) in [(Pane::Song, scenes.clone()), (Pane::Scene, vec![])] {
        let offered = choices(&a, pane)
            .into_iter()
            .map(|(part, _)| part)
            .collect::<Vec<_>>();
        assert!(!offered.contains(&capture[0]), "{pane:?}");
        let (offered_patterns, rest): (Vec<_>, Vec<_>) = offered
            .into_iter()
            .partition(|p| matches!(p, PatternOrScene::Pattern(_)));
        let mut offered_patterns = offered_patterns
            .into_iter()
            .filter_map(|p| match p {
                PatternOrScene::Pattern(id) => Some(id),
                PatternOrScene::Scene(_) => None,
            })
            .collect::<Vec<_>>();
        offered_patterns.sort();
        assert_eq!(offered_patterns, patterns, "{pane:?}");
        assert_eq!(rest, offered_scenes, "{pane:?}");
    }
}

#[test]
fn the_palette_finds_the_keyboard_by_recording_words() {
    let (a, _, _, _, _) = verification_song();
    for screen in [Screen::Main, Screen::Song] {
        let defs = crate::view::commands::descriptions::tests::displayed(
            crate::view::commands::definitions(&a, screen),
        );
        let mut q = crate::view::commands::SearchQuery::default();
        for text in ["rec", "recording", "録音"] {
            q.update(text);
            assert!(
                defs.iter()
                    .any(|c| c.id == "pattern.record" && c.matches_query(&q)),
                "{screen:?} {text}"
            );
        }
    }
}

#[test]
fn the_editor_windows_stay_usable_while_the_keyboard_is_open() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let keyboard = crate::view::recording::window::viewport();
    let before = normal(&a);
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
    // A discarded take adds no pattern beside the Capture of its keys.
    focus(&c, keyboard);
    pad(&mut a, &c, keyboard, Key::Space);
    pad(&mut a, &c, keyboard, Key::Z);
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::SHIFT);
    assert!(a.patterns.recording.is_open());
    assert_eq!(normal(&a), before);
}

#[test]
fn clicking_a_capture_note_only_auditions_it() {
    let (mut a, c, _, _, _) = verification_song();
    let keyboard = crate::view::recording::window::viewport();
    key(&mut a, &c, Pane::Song.viewport(), Key::R, Modifiers::SHIFT);
    pad(&mut a, &c, keyboard, Key::Z);
    key(&mut a, &c, keyboard, Key::Escape, Modifiers::NONE);
    let capture = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .values()
        .find(|p| p.kind == cat_smf_editor::score::pattern::Kind::Capture)
        .map(|p| p.id)
        .unwrap();
    crate::view::patterns::library::edit(&mut a, capture).unwrap();
    let track = a.track;
    let (row, event) = {
        let l = a.loaded.as_ref().unwrap();
        let event = l.visible().tracks[track]
            .events
            .iter()
            .position(cat_smf_editor::score::note_view::note_on)
            .unwrap();
        let rows = &l.display_rows[track];
        let row = (0..rows.len())
            .find(|&r| {
                matches!(rows.get(r), cat_smf_editor::score::display_rows::DisplayRow::Event(i) if i == event)
            })
            .unwrap();
        (row, event)
    };
    // The event list's click: an edit request on the cell, then the selection.
    crate::view::note_edit::begin(
        &mut a,
        crate::view::note_edit::Field::Pitch,
        track,
        event,
        egui::Rect::from_min_size(egui::pos2(400.0, 200.0), egui::vec2(80.0, 25.0)),
        "C4",
    );
    crate::cursor::moves::select(&mut a, row);
    assert_eq!(a.load_error, None);
    assert!(a.note_edit.is_none());
    assert_eq!(a.audition.auditioned, vec![60]);
}
