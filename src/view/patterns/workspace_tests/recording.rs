use super::*;

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
        // While this viewport owns recording, root editing is suspended too.
        key(&mut a, &c, egui::ViewportId::ROOT, Key::I, Modifiers::NONE);
        assert!(!a.mml_input.open);
        key(&mut a, &c, pane.viewport(), Key::C, Modifiers::NONE);
        key(&mut a, &c, pane.viewport(), Key::E, Modifiers::NONE);
        key(&mut a, &c, pane.viewport(), Key::Escape, Modifiers::NONE);
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
