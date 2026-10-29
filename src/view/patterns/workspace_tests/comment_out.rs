use super::*;
fn collection(a: &App) -> scene::Collection {
    scene::read(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn song_disabled(a: &App, lane: u64, measure: u64) -> bool {
    collection(a)
        .song
        .iter()
        .find(|p| p.lane == lane && p.measure == measure)
        .unwrap()
        .disabled
}
fn scene_disabled(a: &App, scene: u64, lane: u64, measure: u64) -> bool {
    collection(a)
        .scenes
        .iter()
        .find(|s| s.id == scene)
        .unwrap()
        .cells
        .iter()
        .find(|c| c.lane == lane && c.measure + 1 == measure)
        .unwrap()
        .disabled
}
fn notes(a: &App, scene: Option<u64>) -> usize {
    crate::play::arrangement::prepare(a, scene)
        .unwrap()
        .parts
        .iter()
        .map(|p| p.2.notes.len())
        .sum()
}
fn comment_out(a: &mut App, c: &egui::Context, p: Pane) {
    key(a, c, p.viewport(), Key::Slash, Modifiers::CTRL);
}
fn right_cell(a: &mut App, c: &egui::Context, p: Pane, row: usize, measure: u64) {
    let id = egui::Id::new((p.title(), "cell", row, measure));
    frame(a, c, p.viewport(), vec![]);
    let pos = c.data(|d| d.get_temp::<egui::Rect>(id)).unwrap().center();
    for pressed in [true, false] {
        frame(
            a,
            c,
            p.viewport(),
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
/// Alpha of the name drawn inside a matrix cell.
fn label_alpha(c: &egui::Context, p: Pane, row: usize, measure: u64) -> u8 {
    let rect = c
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new((p.title(), "cell", row, measure))))
        .unwrap();
    let shapes = c
        .data(|d| {
            d.get_temp::<Vec<egui::epaint::ClippedShape>>(egui::Id::new((
                "workspace-shapes",
                p.viewport(),
            )))
        })
        .unwrap();
    shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if rect.contains(t.pos) && !t.galley.is_empty() => {
                Some(t.fallback_color.a())
            }
            _ => None,
        })
        .expect("cell label")
}
#[test]
fn ctrl_slash_comments_out_song_placement_undoably_and_skips_empty_cells() {
    let (mut a, c, _, _, _) = verification_song();
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    let row = workspace::lanes(&collection(&a), None)
        .iter()
        .position(|l| *l == lane)
        .unwrap();
    cell(&mut a, &c, Pane::Song, lane, 3);
    let before = collection(&a);
    let bright = label_alpha(&c, Pane::Song, row, 3);
    assert_eq!(notes(&a, None), 3);
    comment_out(&mut a, &c, Pane::Song);
    assert!(song_disabled(&a, lane, 3));
    assert_eq!(notes(&a, None), 2);
    assert_eq!(collection(&a).song_length, before.song_length);
    frame(&mut a, &c, Pane::Song.viewport(), vec![]);
    assert!(label_alpha(&c, Pane::Song, row, 3) < bright);
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(collection(&a), before);
    comment_out(&mut a, &c, Pane::Song);
    assert!(song_disabled(&a, lane, 3));
    comment_out(&mut a, &c, Pane::Song);
    assert_eq!(collection(&a), before);
    assert_eq!(notes(&a, None), 3);
    // An empty cell neither changes the song nor adds an undo step.
    comment_out(&mut a, &c, Pane::Song);
    cell(&mut a, &c, Pane::Song, lane, 5);
    let disabled = bytes(&a);
    comment_out(&mut a, &c, Pane::Song);
    assert_eq!(bytes(&a), disabled);
    assert_eq!(a.patterns.workspace.error, None);
    key(&mut a, &c, Pane::Song.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(collection(&a), before);
}
#[test]
fn ctrl_slash_comments_out_scene_cell_and_palette_finds_it() {
    let (mut a, c, _, _, scene) = verification_song();
    let lanes = collection(&a).scenes[0].lanes.clone();
    let before = collection(&a);
    cell(&mut a, &c, Pane::Scene, lanes[0], 1);
    assert_eq!(notes(&a, Some(scene)), 2);
    comment_out(&mut a, &c, Pane::Scene);
    assert!(scene_disabled(&a, scene, lanes[0], 1));
    assert_eq!(notes(&a, Some(scene)), 1);
    key(&mut a, &c, Pane::Scene.viewport(), Key::Z, Modifiers::CTRL);
    assert_eq!(collection(&a), before);
    command(&mut a, &c, Pane::Scene.viewport(), "comment out placement");
    assert!(scene_disabled(&a, scene, lanes[0], 1));
    comment_out(&mut a, &c, Pane::Scene);
    assert_eq!(collection(&a), before);
}
#[test]
fn right_click_selects_and_comments_out_the_placement_under_it() {
    let (mut a, c, _, _, _) = verification_song();
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    let before = collection(&a);
    // Measure 2 is inside the two-measure Scene block starting at measure 1.
    right_cell(&mut a, &c, Pane::Song, 0, 2);
    assert_eq!(a.patterns.workspace.song_cell.measure, 2);
    assert!(song_disabled(&a, lane, 1));
    assert_eq!(notes(&a, None), 1);
    right_cell(&mut a, &c, Pane::Song, 0, 2);
    assert_eq!(collection(&a), before);
    let unchanged = bytes(&a);
    right_cell(&mut a, &c, Pane::Song, 0, 6);
    assert_eq!(bytes(&a), unchanged);
}
#[test]
fn comment_out_while_playing_stops_like_other_placement_edits() {
    let (mut a, c, _, _, _) = verification_song();
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    cell(&mut a, &c, Pane::Song, lane, 3);
    // The click plays from measure 3; the whole song is what an edit must rebuild.
    crate::play::transport::stop(&mut a);
    crate::play::arrangement::start(&mut a, None);
    let pending = |a: &App| {
        a.playback
            .pending
            .as_ref()
            .unwrap()
            .parts
            .iter()
            .map(|p| p.2.notes.len())
            .sum::<usize>()
    };
    assert_eq!(pending(&a), 3);
    // A test frame clears the pending start, so run the key's command
    // directly while the transport state is still observable.
    let toggle = crate::view::commands::definitions(&a, crate::view::commands::Screen::Song)
        .into_iter()
        .find(|c| c.id == "arrangement-comment-out")
        .unwrap();
    crate::view::commands::run_direct(&mut a, &c, &toggle);
    assert!(a.playback.pending.is_none() && a.playback.active.is_none());
    crate::play::arrangement::start(&mut a, None);
    assert_eq!(pending(&a), 2);
}
#[test]
fn query_ime_and_dialog_keep_ctrl_slash() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let lane = workspace::lane(&a, Pane::Song).unwrap();
    cell(&mut a, &c, Pane::Song, lane, 3);
    let before = bytes(&a);
    click(
        &mut a,
        &c,
        song,
        egui::Id::new((Pane::Song.title(), "query")),
    );
    frame(
        &mut a,
        &c,
        song,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
    );
    comment_out(&mut a, &c, Pane::Song);
    assert_eq!(bytes(&a), before);
    frame(
        &mut a,
        &c,
        song,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    command(&mut a, &c, song, "edit scene");
    assert!(a.patterns.workspace.input_open());
    comment_out(&mut a, &c, Pane::Song);
    assert_eq!(bytes(&a), before);
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
    // Leaving the filter keeps candidate focus; hidden cells cannot be edited.
    comment_out(&mut a, &c, Pane::Song);
    assert_eq!(bytes(&a), before);
    // Clear the filter's IME preedit before choosing a candidate again.
    click(
        &mut a,
        &c,
        song,
        egui::Id::new((Pane::Song.title(), "query")),
    );
    key(&mut a, &c, song, Key::A, Modifiers::CTRL);
    key(&mut a, &c, song, Key::Backspace, Modifiers::NONE);
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    key(&mut a, &c, song, Key::F, Modifiers::NONE);
    assert!(a.patterns.workspace.song_cell.placing);
    comment_out(&mut a, &c, Pane::Song);
    assert!(song_disabled(&a, lane, 3));
}
