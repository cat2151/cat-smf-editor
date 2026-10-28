use super::*;
fn drag(a: &mut App, c: &egui::Context, p: Pane, from: egui::Id, to: egui::Id) {
    let id = p.viewport();
    frame(a, c, id, vec![]);
    let rect = |r| {
        c.data(|d| d.get_temp::<egui::Rect>(r))
            .expect("visible control")
    };
    let (from, to) = (rect(from).center(), rect(to).center());
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    for events in [
        vec![egui::Event::PointerMoved(from), button(from, true)],
        vec![egui::Event::PointerMoved(from + egui::vec2(12.0, 0.0))],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerMoved(to), button(to, false)],
    ] {
        frame(a, c, id, events);
    }
}
fn part(p: Pane, part: PatternOrScene) -> egui::Id {
    egui::Id::new((p.title(), "part", format!("{part:?}")))
}
fn at(p: Pane, row: usize, measure: u64) -> egui::Id {
    egui::Id::new((p.title(), "cell", row, measure))
}
fn clear_scene_query(a: &mut App, c: &egui::Context) {
    let p = Pane::Scene;
    click(a, c, p.viewport(), egui::Id::new((p.title(), "query")));
    key(a, c, p.viewport(), Key::A, Modifiers::CTRL);
    key(a, c, p.viewport(), Key::Backspace, Modifiers::NONE);
    key(a, c, p.viewport(), Key::Escape, Modifiers::NONE);
    assert!(a.patterns.workspace.scene_cell.query.is_empty());
    assert_ne!(workspace::focused_query(c), Some(p));
}
#[test]
fn song_shows_four_rows_and_a_drop_on_the_fourth_saves_lanes_and_adds_four_more() {
    let (mut a, c, _, second, _) = verification_song();
    let collection = |a: &App| scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(collection(&a).song_lanes.len(), 1);
    assert_eq!(workspace::rows(&collection(&a), None), 4);
    let before = bytes(&a);
    drag(
        &mut a,
        &c,
        Pane::Song,
        part(Pane::Song, PatternOrScene::Pattern(second)),
        at(Pane::Song, 3, 5),
    );
    let c2 = collection(&a);
    assert_eq!(c2.song_lanes.len(), 4, "{:?}", a.patterns.workspace.error);
    assert_eq!(
        grid::cell(&c2, None, c2.song_lanes[3], 5),
        Some(PatternOrScene::Pattern(second))
    );
    assert_eq!(workspace::rows(&c2, None), 8);
    assert_eq!(a.patterns.workspace.song_cell.row, 3);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(c
        .data(|d| d.get_temp::<egui::Rect>(at(Pane::Song, 7, 5)))
        .is_some());
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), before);
}
#[test]
fn scene_put_on_an_unsaved_row_saves_lanes_up_to_it() {
    let (mut a, c, first, _, scene) = verification_song();
    workspace::open_scene(&mut a, scene).unwrap();
    let id = Pane::Scene.viewport();
    clear_scene_query(&mut a, &c);
    candidate(&mut a, &c, Pane::Scene, PatternOrScene::Pattern(first));
    row_cell(&mut a, &c, Pane::Scene, 2, 1);
    assert_eq!(workspace::lane(&a, Pane::Scene), None);
    key(&mut a, &c, id, Key::Enter, Modifiers::NONE);
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let s = collection.scenes.iter().find(|s| s.id == scene).unwrap();
    assert_eq!(s.lanes.len(), 3);
    assert_eq!(
        grid::cell(&collection, Some(scene), s.lanes[2], 1),
        Some(PatternOrScene::Pattern(first))
    );
    // Removing on a row that is not saved yet leaves the song alone.
    let before = bytes(&a);
    key(&mut a, &c, id, Key::ArrowDown, Modifiers::NONE);
    key(&mut a, &c, id, Key::Delete, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
}
#[test]
fn p_then_a_in_the_song_window_plays_like_the_root() {
    use crate::play::playback::Mode;
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    for standalone in [true, false] {
        if !standalone {
            crate::play::transport::stop(&mut a);
            a.patterns.editing = None;
        }
        key(&mut a, &c, song, Key::P, Modifiers::NONE);
        assert!(a.playback.open);
        key(&mut a, &c, song, Key::A, Modifiers::NONE);
        assert!(!a.playback.open);
        // Independent editing auditions its body, as `p → a` does in the root.
        let expected = a
            .patterns
            .editing
            .as_ref()
            .map_or(Mode::Arrangement(None), |e| Mode::Pattern(e.pattern));
        assert_eq!(a.playback.pending.as_ref().unwrap().active.mode, expected);
    }
}
#[test]
fn song_click_plays_from_that_measure_to_the_end() {
    use crate::play::playback::Mode;
    let (mut a, c, _, _, _) = verification_song();
    crate::play::transport::stop(&mut a);
    let whole = crate::play::arrangement::prepare(&a, None).unwrap().active;
    row_cell(&mut a, &c, Pane::Song, 0, 3);
    let music = &a.loaded.as_ref().unwrap().music;
    let tick = grid::measure_tick(music, None, 3).unwrap();
    let start = cat_smf_editor::score::audition_timing::TempoMap::new(music, 0).seconds_at(tick);
    let active = &a.playback.pending.as_ref().expect("playing").active;
    assert_eq!(active.mode, Mode::Arrangement(None));
    assert!(start > 0.0);
    assert_eq!(active.start_seconds, start);
    assert_eq!(active.end, whole.end - start);
}
#[test]
fn song_drag_moves_a_placement_and_the_trash_removes_it() {
    let (mut a, c, _, second, scene) = verification_song();
    let collection = |a: &App| scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let before = bytes(&a);
    drag(
        &mut a,
        &c,
        Pane::Song,
        at(Pane::Song, 0, 3),
        at(Pane::Song, 1, 5),
    );
    let c2 = collection(&a);
    assert_eq!(
        grid::cell(&c2, None, c2.song_lanes[0], 3),
        None,
        "{:?}",
        a.patterns.workspace.error
    );
    assert_eq!(
        grid::cell(&c2, None, c2.song_lanes[1], 5),
        Some(PatternOrScene::Pattern(second))
    );
    assert_eq!(
        (
            a.patterns.workspace.song_cell.row,
            a.patterns.workspace.song_cell.measure
        ),
        (1, 5)
    );
    // A drag from inside a multimeasure body moves the whole block.
    drag(
        &mut a,
        &c,
        Pane::Song,
        at(Pane::Song, 0, 2),
        at(Pane::Song, 0, 7),
    );
    let c2 = collection(&a);
    assert_eq!(
        grid::cell(&c2, None, c2.song_lanes[0], 7),
        Some(PatternOrScene::Scene(scene))
    );
    drag(
        &mut a,
        &c,
        Pane::Song,
        at(Pane::Song, 0, 7),
        egui::Id::new((Pane::Song.title(), "trash")),
    );
    let c2 = collection(&a);
    assert_eq!(c2.song.len(), 1);
    assert_eq!(c2.song[0].part, PatternOrScene::Pattern(second));
    for _ in 0..3 {
        key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    }
    assert_eq!(bytes(&a), before);
}
#[test]
fn song_click_on_a_pattern_shows_it_in_the_event_list_and_keeps_the_song_window() {
    let (mut a, c, _, second, _) = verification_song();
    crate::play::transport::stop(&mut a);
    row_cell(&mut a, &c, Pane::Song, 0, 3);
    assert!(
        a.playback.pending.is_some(),
        "{:?} {:?}",
        a.load_error,
        a.patterns.workspace.error
    );
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    let editing = a
        .patterns
        .editing
        .as_ref()
        .expect("event list shows a body");
    assert_eq!(editing.pattern, second, "{:?}", a.patterns.workspace.error);
    assert!(editing.placement.is_some());
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", Pane::Song.viewport()))))
        .unwrap_or(false));
    assert_eq!(a.patterns.workspace.return_to, None);
}
#[test]
fn song_click_on_a_scene_block_opens_it_in_the_scene_window_and_plays() {
    use crate::play::playback::Mode;
    let visible = |c: &egui::Context, p: Pane| {
        c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", p.viewport()))))
            .unwrap_or(false)
    };
    // The block spans measures 1 and 2; either measure opens the same scene.
    for measure in [1, 2] {
        let (mut a, c, _, _, scene) = verification_song();
        crate::play::transport::stop(&mut a);
        a.patterns.workspace.scene_window = false;
        a.patterns.workspace.scene = None;
        frame(&mut a, &c, Pane::Song.viewport(), vec![]);
        assert!(!visible(&c, Pane::Scene));
        row_cell(&mut a, &c, Pane::Song, 0, measure);
        assert!(a.patterns.workspace.scene_window, "measure {measure}");
        assert_eq!(a.patterns.workspace.scene, Some(scene));
        assert_eq!(a.patterns.workspace.error, None);
        assert!(a.patterns.workspace.scene_cell.focus);
        let active = &a.playback.pending.as_ref().expect("playing").active;
        assert_eq!(active.mode, Mode::Arrangement(None));
        frame(&mut a, &c, Pane::Song.viewport(), vec![]);
        assert!(visible(&c, Pane::Scene), "measure {measure}");
    }
}
#[test]
fn opening_a_scene_keeps_the_transport() {
    let (mut a, _c, _, _, scene) = verification_song();
    crate::play::arrangement::start(&mut a, None);
    assert!(a.playback.pending.is_some());
    workspace::open_scene(&mut a, scene).unwrap();
    assert!(a.playback.pending.is_some());
}
#[test]
fn song_click_on_an_empty_cell_leaves_the_scene_window_closed() {
    let (mut a, c, _, _, _) = verification_song();
    a.patterns.workspace.scene_window = false;
    a.patterns.workspace.scene = None;
    row_cell(&mut a, &c, Pane::Song, 1, 1);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(!a.patterns.workspace.scene_window);
    assert_eq!(a.patterns.workspace.scene, None);
    assert!(!c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", Pane::Scene.viewport()))))
        .unwrap_or(false));
}
fn visible(c: &egui::Context, p: Pane) -> bool {
    c.data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-visible", p.viewport()))))
        .unwrap_or(false)
}
#[test]
#[should_panic(expected = "visible control")]
fn candidate_click_rejects_a_filtered_pattern_with_a_stale_rect() {
    let (mut a, c, first, _, _) = verification_song();
    assert_eq!(a.patterns.workspace.scene_cell.query, "Pattern 2");
    assert!(c
        .data(|d| d.get_temp::<egui::Rect>(part(Pane::Scene, PatternOrScene::Pattern(first))))
        .is_some());
    candidate(&mut a, &c, Pane::Scene, PatternOrScene::Pattern(first));
}
#[test]
fn candidate_click_on_a_pattern_opens_it_unbound_and_auditions_it() {
    use crate::play::playback::Mode;
    // The fixture leaves `first` open, so each pane opens the other pattern.
    for (p, opened) in [(Pane::Song, 1), (Pane::Scene, 0)] {
        let (mut a, c, first, second, _) = verification_song();
        let id = [first, second][opened];
        if p == Pane::Scene {
            library::edit(&mut a, second).unwrap();
            clear_scene_query(&mut a, &c);
        }
        crate::play::transport::stop(&mut a);
        candidate(&mut a, &c, p, PatternOrScene::Pattern(id));
        let active = &a.playback.pending.as_ref().expect("auditioning").active;
        assert_eq!(active.mode, Mode::Pattern(id), "{p:?}");
        let editing = a.patterns.editing.as_ref().expect("event list shows it");
        assert_eq!(
            (editing.pattern, editing.placement),
            (id, None),
            "{p:?} {:?}",
            a.patterns.workspace.error
        );
        assert_eq!(a.patterns.workspace.return_to, None);
        frame(&mut a, &c, p.viewport(), vec![]);
        assert!(visible(&c, p), "{p:?}");
    }
}
#[test]
fn candidate_click_on_a_scene_opens_it_in_the_scene_window() {
    use crate::play::playback::Mode;
    let (mut a, c, _, _, scene) = verification_song();
    crate::play::transport::stop(&mut a);
    a.patterns.workspace.scene_window = false;
    a.patterns.workspace.scene = None;
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(!visible(&c, Pane::Scene));
    candidate(&mut a, &c, Pane::Song, PatternOrScene::Scene(scene));
    assert_eq!(a.patterns.workspace.scene, Some(scene));
    assert!(a.patterns.workspace.scene_cell.focus);
    let active = &a.playback.pending.as_ref().expect("playing").active;
    assert_eq!(active.mode, Mode::Arrangement(Some(scene)));
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(visible(&c, Pane::Scene));
}
#[test]
fn page_keys_and_candidate_drags_open_nothing() {
    let (mut a, c, _, second, _) = verification_song();
    a.patterns.workspace.scene_window = false;
    a.patterns.workspace.scene = None;
    let editing = a.patterns.editing.clone();
    let song = Pane::Song.viewport();
    let mut seen = Vec::new();
    for k in [Key::PageDown, Key::PageDown, Key::PageDown, Key::PageUp] {
        key(&mut a, &c, song, k, Modifiers::NONE);
        seen.push(a.patterns.workspace.song_cell.choice);
    }
    assert!(seen
        .iter()
        .any(|c| matches!(c, Some(PatternOrScene::Scene(_)))));
    assert!(seen.contains(&Some(PatternOrScene::Pattern(second))));
    assert_eq!(a.patterns.editing, editing);
    assert!(!a.patterns.workspace.scene_window);
    assert_eq!(a.patterns.workspace.scene, None);
    drag(
        &mut a,
        &c,
        Pane::Song,
        part(Pane::Song, PatternOrScene::Pattern(second)),
        at(Pane::Song, 1, 5),
    );
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        grid::cell(&collection, None, collection.song_lanes[1], 5),
        Some(PatternOrScene::Pattern(second)),
        "{:?}",
        a.patterns.workspace.error
    );
    assert_eq!(a.patterns.editing, editing);
}
