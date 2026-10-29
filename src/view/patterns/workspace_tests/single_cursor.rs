use super::*;

fn press(a: &mut App, c: &egui::Context, k: Key) {
    key(a, c, Pane::Song.viewport(), k, Modifiers::NONE);
}

fn cursor_count(a: &mut App, c: &egui::Context) -> usize {
    frame(a, c, Pane::Song.viewport(), vec![]);
    let shapes = c.data(|d| {
        d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
            "workspace-shapes",
            Pane::Song.viewport(),
        )))
        .unwrap()
    });
    shapes
        .iter()
        .filter(|s| {
            matches!(&s.shape,
                egui::epaint::Shape::Rect(r) if r.stroke.width == 2.0
                    && r.rect.height() == 30.0
            )
        })
        .count()
}

#[test]
fn song_single_cursor_select_place_repeat_return_and_close_with_only_keys() {
    let (mut a, c, first, _, _) = verification_song();
    press(&mut a, &c, Key::Escape);
    assert!(!a.patterns.workspace.song_cell.placing);
    assert_eq!(cursor_count(&mut a, &c), 1);
    let before = bytes(&a);
    let position = (
        a.patterns.workspace.song_cell.row,
        a.patterns.workspace.song_cell.measure,
    );
    press(&mut a, &c, Key::K);
    assert_eq!(
        a.patterns.workspace.song_cell.choice,
        Some(PatternOrScene::Pattern(first))
    );
    assert_eq!(
        (
            a.patterns.workspace.song_cell.row,
            a.patterns.workspace.song_cell.measure
        ),
        position
    );
    press(&mut a, &c, Key::F);
    assert!(a.patterns.workspace.song_cell.placing);
    assert_eq!(bytes(&a), before, "choosing a part does not place it");
    assert_eq!(cursor_count(&mut a, &c), 1);
    for k in [Key::L, Key::F, Key::L, Key::Enter, Key::J, Key::F] {
        press(&mut a, &c, k);
    }
    assert!(a.patterns.workspace.song_cell.placing);
    assert_eq!(
        a.patterns.workspace.song_cell.choice,
        Some(PatternOrScene::Pattern(first))
    );
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    for (row, measure) in [(0, 4), (0, 5), (1, 5)] {
        assert_eq!(
            grid::cell(&collection, None, collection.song_lanes[row], measure),
            Some(PatternOrScene::Pattern(first))
        );
    }
    let position = (
        a.patterns.workspace.song_cell.row,
        a.patterns.workspace.song_cell.measure,
    );
    let placed = bytes(&a);
    for k in [Key::PageDown, Key::PageUp, Key::Tab] {
        press(&mut a, &c, k);
    }
    assert_eq!(bytes(&a), placed);
    assert_eq!(
        a.patterns.workspace.song_cell.choice,
        Some(PatternOrScene::Pattern(first))
    );
    press(&mut a, &c, Key::Escape);
    assert!(mode::workspace_open(&a));
    assert_eq!(cursor_count(&mut a, &c), 1);
    press(&mut a, &c, Key::Enter);
    assert_eq!(
        (
            a.patterns.workspace.song_cell.row,
            a.patterns.workspace.song_cell.measure
        ),
        position
    );
    assert_eq!(bytes(&a), placed);
    press(&mut a, &c, Key::Escape);
    press(&mut a, &c, Key::Escape);
    assert!(!mode::workspace_open(&a));
}

#[test]
fn song_starts_at_candidates_and_filter_keeps_f_and_ime_input() {
    let (mut a, c, _, _, _) = verification_song();
    // A fresh workspace starts on candidates, independently of its saved cell.
    a.patterns.workspace.song_cell = Default::default();
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(!a.patterns.workspace.song_cell.placing);
    assert_eq!(cursor_count(&mut a, &c), 1);
    press(&mut a, &c, Key::F);
    let before = bytes(&a);
    click(
        &mut a,
        &c,
        Pane::Song.viewport(),
        egui::Id::new((Pane::Song.title(), "query")),
    );
    assert!(!a.patterns.workspace.song_cell.placing);
    press(&mut a, &c, Key::F);
    assert!(!a.patterns.workspace.song_cell.placing);
    frame(
        &mut a,
        &c,
        Pane::Song.viewport(),
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
    );
    press(&mut a, &c, Key::Enter);
    assert!(!a.patterns.workspace.song_cell.placing);
    assert_eq!(bytes(&a), before);
    frame(
        &mut a,
        &c,
        Pane::Song.viewport(),
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    press(&mut a, &c, Key::Escape);
    assert!(mode::workspace_open(&a));
    assert!(!a.patterns.workspace.song_cell.placing);
}

#[test]
fn song_drop_selects_the_part_and_cell_for_repeated_keyboard_placement() {
    let (mut a, c, first, _, _) = verification_song();
    press(&mut a, &c, Key::Escape);
    arrange::drag(
        &mut a,
        &c,
        Pane::Song,
        arrange::part(Pane::Song, PatternOrScene::Pattern(first)),
        egui::Id::new((Pane::Song.title(), "cell", 1usize, 5u64)),
    );
    assert!(a.patterns.workspace.song_cell.placing);
    assert_eq!(cursor_count(&mut a, &c), 1);
    press(&mut a, &c, Key::L);
    press(&mut a, &c, Key::F);
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    for measure in [5, 6] {
        assert_eq!(
            grid::cell(&collection, None, collection.song_lanes[1], measure),
            Some(PatternOrScene::Pattern(first))
        );
    }
}
