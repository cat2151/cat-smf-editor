use super::arrange::{drag, part};
use super::*;

#[test]
fn song_candidate_trash_removes_body_and_all_occurrences_in_one_undo() {
    let (mut a, c, first, second, scene) = verification_song();
    let before = bytes(&a);
    let first_body = body(&a, first);
    drag(
        &mut a,
        &c,
        Pane::Song,
        part(Pane::Song, PatternOrScene::Pattern(second)),
        egui::Id::new((Pane::Song.title(), "trash")),
    );
    assert_eq!(a.patterns.workspace.error, None);
    let music = &a.loaded.as_ref().unwrap().music;
    assert!(!Catalog::read(music).unwrap().patterns.contains_key(&second));
    let collection = scene::read(music).unwrap();
    assert_eq!(collection.song.len(), 1);
    assert_eq!(collection.song[0].part, PatternOrScene::Scene(scene));
    assert!(collection
        .scenes
        .iter()
        .all(|s| s.cells.iter().all(|cell| cell.pattern != second)));
    assert_eq!(body(&a, first), first_body);
    assert!(a.patterns.editing.is_none());
    assert_ne!(a.patterns.selected, Some(second));
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), before);
}

#[test]
fn song_candidate_can_be_selected_deleted_and_restored_with_only_keys() {
    let (mut a, c, first, second, _) = verification_song();
    let before = bytes(&a);
    // Return from placement, then select the previous candidate.
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Escape,
        Modifiers::NONE,
    );
    key(&mut a, &c, Pane::Song.viewport(), Key::K, Modifiers::NONE);
    assert_eq!(
        a.patterns.workspace.song_cell.choice,
        Some(PatternOrScene::Pattern(first))
    );
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Delete,
        Modifiers::SHIFT,
    );
    let catalog = Catalog::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert!(!catalog.patterns.contains_key(&first));
    assert!(catalog.patterns.contains_key(&second));
    assert!(!crate::view::modal::is_open(&a));
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), before);
}

#[test]
fn scene_candidates_and_cancelled_drags_do_not_delete_patterns() {
    let (mut a, c, _, second, scene) = verification_song();
    let before = bytes(&a);
    for candidate in [
        PatternOrScene::Scene(scene),
        PatternOrScene::Pattern(second),
    ] {
        let target = if matches!(candidate, PatternOrScene::Scene(_)) {
            egui::Id::new((Pane::Song.title(), "trash"))
        } else {
            // The search field is outside every drop target.
            egui::Id::new((Pane::Song.title(), "query"))
        };
        drag(&mut a, &c, Pane::Song, part(Pane::Song, candidate), target);
        assert_eq!(bytes(&a), before);
    }
}

#[test]
fn delete_pattern_picker_is_no_longer_registered() {
    let (a, _, _, _, _) = verification_song();
    for screen in [Screen::Main, Screen::Song, Screen::Scene] {
        let mut commands = Vec::new();
        collect(&a, screen, &mut commands);
        assert!(commands.iter().all(|c| c.id != "delete-pattern"));
    }
}

#[test]
fn shift_delete_in_the_filter_does_not_delete_the_candidate() {
    let (mut a, c, _, _, _) = verification_song();
    let before = bytes(&a);
    let p = Pane::Song;
    click(
        &mut a,
        &c,
        p.viewport(),
        egui::Id::new((p.title(), "query")),
    );
    text(&mut a, &c, p.viewport(), "Pattern");
    key(&mut a, &c, p.viewport(), Key::Delete, Modifiers::SHIFT);
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, p.viewport(), Key::Escape, Modifiers::NONE);
    assert_ne!(workspace::focused_query(&c), Some(p));
}

#[test]
fn deleting_the_last_pattern_and_a_stale_candidate_is_atomic() {
    let (mut a, c, first, second, _) = verification_song();
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Escape,
        Modifiers::NONE,
    );
    for id in [first, second] {
        a.patterns.workspace.song_cell.choice = Some(PatternOrScene::Pattern(id));
        key(
            &mut a,
            &c,
            Pane::Song.viewport(),
            Key::Delete,
            Modifiers::SHIFT,
        );
    }
    let before = bytes(&a);
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .is_empty());
    a.patterns.workspace.song_cell.choice = Some(PatternOrScene::Pattern(first));
    key(
        &mut a,
        &c,
        Pane::Song.viewport(),
        Key::Delete,
        Modifiers::SHIFT,
    );
    assert!(a.patterns.workspace.error.is_some());
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert!(Catalog::read(&a.loaded.as_ref().unwrap().music)
        .unwrap()
        .patterns
        .contains_key(&second));
}
