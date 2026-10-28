use super::*;
fn prepared_notes(a: &App) -> Vec<(u8, f64)> {
    let p = crate::play::arrangement::prepare(a, None).unwrap();
    let mut n = p
        .parts
        .iter()
        .flat_map(|p| p.2.notes.iter().map(|n| (n.pitch, n.start)))
        .collect::<Vec<_>>();
    n.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    n
}
#[test]
fn workspace_keyboard_reorders_scene_blocks_via_independent_lane_and_undo() {
    let (mut a, c, first, second, aid) = verification_song();
    let song = Pane::Song.viewport();
    let source = a.track;
    let pa = body(&a, first)[0];
    let pb = body(&a, second)[0];
    create_scene(&mut a, &c, "B", "1");
    let bid = a.patterns.workspace.scene.unwrap();
    candidate(&mut a, &c, Pane::Scene, PatternOrScene::Pattern(second));
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    candidate(&mut a, &c, Pane::Song, PatternOrScene::Scene(bid));
    cell(&mut a, &c, Pane::Song, lane, 3);
    key(&mut a, &c, song, Key::Enter, Modifiers::NONE);
    assert_eq!(prepared_notes(&a), vec![(pa, 0.0), (pb, 2.0), (pb, 4.0)]);
    cell(&mut a, &c, Pane::Song, lane, 1);
    key(&mut a, &c, song, Key::ArrowDown, Modifiers::CTRL);
    let other = workspace::lane(&a, Pane::Song).unwrap();
    assert_ne!(lane, other);
    cell(&mut a, &c, Pane::Song, lane, 3);
    for _ in 0..2 {
        key(&mut a, &c, song, Key::ArrowLeft, Modifiers::CTRL);
    }
    let parallel = prepared_notes(&a);
    assert_eq!(parallel.len(), 3);
    assert_eq!(parallel.iter().filter(|n| n.1 == 0.0).count(), 2);
    cell(&mut a, &c, Pane::Song, other, 1);
    key(&mut a, &c, song, Key::ArrowRight, Modifiers::CTRL);
    let before = bytes(&a);
    key(&mut a, &c, song, Key::ArrowUp, Modifiers::CTRL);
    assert_eq!(prepared_notes(&a), vec![(pb, 0.0), (pa, 2.0), (pb, 4.0)]);
    assert_eq!(a.track, source);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, second);
    key(&mut a, &c, song, Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), before);
    cell(&mut a, &c, Pane::Song, other, 2);
    key(&mut a, &c, song, Key::ArrowUp, Modifiers::CTRL);
    let placed = bytes(&a);
    key(&mut a, &c, song, Key::Delete, Modifiers::NONE);
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(collection.song.len(), 1);
    assert!(collection.scenes.iter().any(|s| s.id == aid));
    key(&mut a, &c, song, Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), placed);
    key(&mut a, &c, song, Key::Space, Modifiers::NONE);
    assert!(a.playback.pending.is_some());
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    assert!(a.note_mode && !mode::workspace_open(&a));
    assert!(a.playback.pending.is_none() && a.playback.active.is_none());
    assert!(!mode::actual_source(&a, a.track));
}
#[test]
fn workspace_edit_body_returns_to_original_cell_and_rebuilds_shared_occurrences() {
    let (mut a, c, first, second, scene) = verification_song();
    let song_lane = workspace::lane(&a, Pane::Song).unwrap();
    candidate(&mut a, &c, Pane::Song, PatternOrScene::Scene(scene));
    cell(&mut a, &c, Pane::Song, song_lane, 4);
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Enter,
        Modifiers::NONE,
    );
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let lane = collection.scenes[0].lanes[0];
    let other = body(&a, second);
    cell(&mut a, &c, Pane::Scene, lane, 1);
    key(&mut a, &c, Pane::Scene.viewport(), Key::E, Modifiers::NONE);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, first);
    assert_eq!(a.patterns.workspace.return_to, Some(Pane::Scene));
    let root = egui::ViewportId::ROOT;
    key(&mut a, &c, root, Key::Home, Modifiers::NONE);
    key(&mut a, &c, root, Key::E, Modifiers::NONE);
    key(&mut a, &c, root, Key::P, Modifiers::NONE);
    frame(&mut a, &c, root, vec![]);
    assert!(a.note_edit.is_some());
    key(&mut a, &c, root, Key::A, Modifiers::CTRL);
    text(&mut a, &c, root, "D4");
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(a.note_edit.is_none());
    assert_eq!(body(&a, second), other);
    assert_eq!(
        scene::read(&a.loaded.as_ref().unwrap().music).unwrap(),
        collection
    );
    let pitch = body(&a, first)[0];
    let p = crate::play::arrangement::prepare(&a, Some(scene)).unwrap();
    assert!(p
        .parts
        .iter()
        .any(|p| p.2.notes.iter().any(|n| n.pitch == pitch)));
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert_eq!(workspace::lane(&a, Pane::Scene), Some(lane));
    assert_eq!(a.patterns.workspace.scene_cell.measure, 1);
    assert_eq!(a.patterns.workspace.return_to, None);
    assert!(c
        .data(|d| d.get_temp::<bool>(egui::Id::new(("workspace-focus", Pane::Scene.viewport()))))
        .unwrap_or(false));
    // Shared transport boundary: edits while Song is prepared keep Song scope,
    // sound, global tempo and all occurrences, independent of the root cursor.
    crate::play::arrangement::start(&mut a, None);
    let track = a.track;
    let on = a.loaded.as_ref().unwrap().visible().tracks[track]
        .events
        .iter()
        .position(cat_smf_editor::score::note_view::note_on)
        .unwrap();
    a.edit_music(|m| cat_smf_editor::edit::note_value::set_pitch(&mut m.tracks[track], on, 70))
        .unwrap();
    let p = a.playback.pending.as_ref().unwrap();
    assert_eq!(
        p.active.mode,
        crate::play::playback::Mode::Arrangement(None)
    );
    assert_eq!(p.active.end, 10.0);
    assert_eq!(
        p.parts
            .iter()
            .flat_map(|p| p.2.notes.iter())
            .filter(|n| n.pitch == 70)
            .count(),
        2
    );
    let suspended = crate::play::selection_playback::suspend(&mut a).unwrap();
    a.loaded.as_mut().unwrap().patches[track].layers[0]
        .settings
        .octave = 3;
    crate::play::selection_playback::restore(&mut a, suspended);
    assert!(a
        .playback
        .pending
        .as_ref()
        .unwrap()
        .parts
        .iter()
        .all(|p| p.1.layers[0].settings.octave == 3));
    crate::play::transport::timeline::seek(&mut a, 0, 960);
    assert_eq!(
        a.playback.pending.as_ref().unwrap().active.mode,
        crate::play::playback::Mode::Arrangement(None)
    );
}

#[test]
fn multimeasure_block_middle_keyboard_edit_and_delete_resolve_the_placement_id() {
    let (mut a, c, _, _, _) = verification_song();
    let mut ids = (0, 0, 0);
    a.edit_tracks(|m| {
        let catalog = Catalog::read(m)?;
        let source = *catalog.sources.first().unwrap();
        ids.0 = cat_smf_editor::score::pattern::operations::create(m, source, None)?;
        cat_smf_editor::score::pattern::operations::resize(
            m,
            ids.0,
            vec![
                scene::Meter {
                    numerator: 4,
                    exponent: 2,
                    ticks: 1920
                };
                2
            ],
        )?;
        ids.1 = grid::create_scene(m, "Long", 4, 1)?;
        ids.2 = scene::read(m)?
            .scenes
            .iter()
            .find(|s| s.id == ids.1)
            .unwrap()
            .lanes[0];
        grid::place(
            m,
            Some(ids.1),
            ids.2,
            1,
            Some(PatternOrScene::Pattern(ids.0)),
        )
    })
    .unwrap();
    workspace::open_scene(&mut a, ids.1).unwrap();
    let placement = scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .scenes
        .iter()
        .find(|s| s.id == ids.1)
        .unwrap()
        .cells[0]
        .id;
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::ArrowRight,
        Modifiers::NONE,
    );
    assert_eq!(a.patterns.workspace.scene_cell.measure, 2);
    key(&mut a, &c, Pane::Scene.viewport(), Key::E, Modifiers::NONE);
    let edit = a.patterns.editing.as_ref().unwrap();
    assert_eq!(edit.pattern, ids.0);
    assert_eq!(edit.placement, Some(placement));
    key(
        &mut a,
        &c,
        egui::ViewportId::ROOT,
        Key::Escape,
        Modifiers::NONE,
    );
    assert_eq!(a.patterns.workspace.scene_cell.measure, 2);
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::Delete,
        Modifiers::NONE,
    );
    assert!(scene::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .scenes
        .iter()
        .find(|s| s.id == ids.1)
        .unwrap()
        .cells
        .is_empty());
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .contains_key(&ids.0));
}
#[test]
fn shift_e_edits_the_chosen_unplaced_part_and_escape_returns_to_song() {
    let (mut a, c, first, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let unplaced = add(&mut a, &c, "o5 l1 c");
    library::edit(&mut a, first).unwrap();
    let before = bytes(&a);
    for _ in 0..workspace::choices(&a, Pane::Song).len() {
        if a.patterns.workspace.selection(Pane::Song).choice
            == Some(PatternOrScene::Pattern(unplaced))
        {
            break;
        }
        key(&mut a, &c, song, Key::PageDown, Modifiers::NONE);
    }
    assert_eq!(
        a.patterns.workspace.selection(Pane::Song).choice,
        Some(PatternOrScene::Pattern(unplaced))
    );
    key(&mut a, &c, song, Key::E, Modifiers::SHIFT);
    let edit = a.patterns.editing.as_ref().unwrap();
    assert!(edit.standalone && edit.pattern == unplaced);
    assert_eq!(a.patterns.workspace.return_to, Some(Pane::Song));
    assert_eq!(bytes(&a), before, "opening a part never places or edits it");
    let root = egui::ViewportId::ROOT;
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert_eq!(a.patterns.workspace.return_to, None);
    assert!(mode::workspace_open(&a));
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, unplaced);
}
#[test]
fn song_n_chooses_container_from_current_and_cancel_keeps_song() {
    let (mut a, c, first, _, _) = verification_song();
    let owner = Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns[&first]
        .container;
    a.edit_tracks(|m| cat_smf_editor::score::pattern::operations::create_source(m).map(|_| ()))
        .unwrap();
    let song = Pane::Song.viewport();
    let root = egui::ViewportId::ROOT;
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    cell(&mut a, &c, Pane::Song, lane, 3);
    let before = bytes(&a);
    key(&mut a, &c, song, Key::N, Modifiers::NONE);
    assert!(a.patterns.confirming_creation());
    assert_eq!(a.patterns.container, Some(owner));
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.confirming_creation());
    assert_eq!(bytes(&a), before);
    assert_eq!(a.patterns.workspace.selection(Pane::Song).measure, 3);
    key(&mut a, &c, song, Key::N, Modifiers::NONE);
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(
        catalog.patterns[&a.patterns.selected.unwrap()].container,
        owner
    );
    a.undo_music().unwrap();
    a.mml_input.open = false;
    key(&mut a, &c, song, Key::N, Modifiers::NONE);
    key(&mut a, &c, root, Key::ArrowDown, Modifiers::NONE);
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let created = &catalog.patterns[&a.patterns.selected.unwrap()];
    assert_ne!(created.container, owner);
    frame(&mut a, &c, root, vec![]);
    assert!(a.mml_input.open, "{:?}", a.load_error);
}
#[test]
fn song_n_offers_new_init_saw_once_the_only_one_holds_notes() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let root = egui::ViewportId::ROOT;
    let tracks = a.loaded.as_ref().unwrap().music.tracks.len();
    let before = bytes(&a);
    key(&mut a, &c, song, Key::N, Modifiers::NONE);
    key(&mut a, &c, root, Key::ArrowDown, Modifiers::NONE);
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    let music = &a.loaded.as_ref().unwrap().music;
    assert_eq!(music.tracks.len(), tracks + 1, "{:?}", a.patterns.error);
    let p = &Catalog::read(music).unwrap().patterns[&a.patterns.selected.unwrap()];
    assert_eq!(music.tracks[p.track].name().as_deref(), Some("Init Saw 2"));
    frame(&mut a, &c, root, vec![]);
    assert!(a.mml_input.open, "{:?}", a.load_error);
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}
#[test]
fn plus_below_parts_opens_the_same_container_choice_in_both_panes() {
    let (mut a, c, _, _, _) = verification_song();
    let root = egui::ViewportId::ROOT;
    let before = bytes(&a);
    for p in [Pane::Song, Pane::Scene] {
        frame(&mut a, &c, p.viewport(), vec![]);
        button(&mut a, &c, p, "new-pattern");
        assert!(a.patterns.confirming_creation(), "{p:?}");
        key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
        assert!(!a.patterns.confirming_creation());
        assert_eq!(bytes(&a), before);
    }
}
