use super::*;
#[test]
fn workspace_form_invalid_ime_click_cancel_and_stale_target_are_atomic() {
    let (mut a, c, _, _, _) = verification_song();
    let scene = Pane::Scene.viewport();
    let before = bytes(&a);
    let edit = a.patterns.editing.clone();
    button(&mut a, &c, Pane::Scene, "create");
    text(&mut a, &c, scene, "空 scene");
    key(&mut a, &c, scene, Key::Tab, Modifiers::NONE);
    key(&mut a, &c, scene, Key::A, Modifiers::CTRL);
    text(&mut a, &c, scene, "0");
    key(&mut a, &c, scene, Key::Enter, Modifiers::NONE);
    assert!(a.patterns.workspace.input_open());
    assert_eq!(bytes(&a), before);
    let root = egui::ViewportId::ROOT;
    key(&mut a, &c, root, Key::I, Modifiers::NONE);
    assert!(!a.mml_input.open);
    assert_eq!(a.patterns.editing, edit);
    key(&mut a, &c, scene, Key::A, Modifiers::CTRL);
    text(&mut a, &c, scene, "2");
    frame(
        &mut a,
        &c,
        scene,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
    );
    key(&mut a, &c, scene, Key::Enter, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    frame(
        &mut a,
        &c,
        scene,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    click(
        &mut a,
        &c,
        scene,
        egui::Id::new(("arrangement-field", "arrangement-length")),
    );
    key(&mut a, &c, scene, Key::A, Modifiers::CTRL);
    text(&mut a, &c, scene, "2");
    click(&mut a, &c, scene, egui::Id::new("arrangement-confirm"));
    assert!(!a.patterns.workspace.input_open());
    let collection = scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    let s = collection
        .scenes
        .iter()
        .find(|s| s.name == "空 scene")
        .unwrap();
    assert_eq!(s.meters.len(), 2);
    assert!(s.cells.is_empty());
    key(&mut a, &c, scene, Key::Z, Modifiers::CTRL);
    assert_eq!(bytes(&a), before);
    let song = Pane::Song.viewport();
    command(&mut a, &c, song, "edit scene");
    text(&mut a, &c, song, "nihongo 日本語");
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    command(&mut a, &c, song, "edit scene");
    a.song_id += 1;
    key(&mut a, &c, song, Key::Enter, Modifiers::NONE);
    assert!(!a.patterns.workspace.input_open());
    assert_eq!(bytes(&a), before);
    assert!(a
        .patterns
        .workspace
        .error
        .as_ref()
        .unwrap()
        .contains("target changed"));
}
#[test]
fn workspace_query_owns_letters_arrows_help_palette_and_scene_deletion() {
    let (mut a, c, _, _, _) = verification_song();
    let song = Pane::Song.viewport();
    let before = bytes(&a);
    let root_edit = a.patterns.editing.clone();
    let selected = (
        a.patterns.workspace.song_cell.row,
        a.patterns.workspace.song_cell.measure,
    );
    click(
        &mut a,
        &c,
        song,
        egui::Id::new((Pane::Song.title(), "query")),
    );
    text(&mut a, &c, song, "Pattern 1");
    frame(
        &mut a,
        &c,
        song,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit("あ".into()))],
    );
    key(&mut a, &c, song, Key::Enter, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    frame(
        &mut a,
        &c,
        song,
        vec![egui::Event::Ime(egui::ImeEvent::Disabled)],
    );
    key(&mut a, &c, song, Key::A, Modifiers::CTRL);
    text(&mut a, &c, song, "nihongo 日本語");
    key(&mut a, &c, song, Key::E, Modifiers::NONE);
    key(&mut a, &c, song, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    assert_eq!(a.patterns.editing, root_edit);
    assert!(!a.mml_input.open);
    assert!(!a.patterns.workspace.song_cell.help);
    assert_eq!(
        (
            a.patterns.workspace.song_cell.row,
            a.patterns.workspace.song_cell.measure
        ),
        selected
    );
    command(&mut a, &c, song, "play song");
    assert!(a.playback.pending.is_some() || a.playback.active.is_some());
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE); // Return focus from the filter.
    frame(&mut a, &c, song, vec![egui::Event::Text("?".into())]);
    assert!(a.patterns.workspace.song_cell.help);
    key(&mut a, &c, song, Key::Escape, Modifiers::NONE);
    assert!(!a.patterns.workspace.song_cell.help);
    button(&mut a, &c, Pane::Scene, "create");
    let mut raw = egui::RawInput {
        events: vec![egui::Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
        ..Default::default()
    };
    eframe::App::raw_input_hook(&mut a, &c, &mut raw);
    assert_eq!(raw.events.len(), 1);
    key(
        &mut a,
        &c,
        Pane::Scene.viewport(),
        Key::Escape,
        Modifiers::NONE,
    );
    let scene = Pane::Scene.viewport();
    key(
        &mut a,
        &c,
        scene,
        Key::P,
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    text(&mut a, &c, scene, "delete scene");
    // The first result is Scene: Remove placement, which also matches the
    // Delete key. Select the distinct Delete scene command before executing.
    key(&mut a, &c, scene, Key::ArrowDown, Modifiers::NONE);
    key(&mut a, &c, scene, Key::Enter, Modifiers::NONE);
    assert_eq!(bytes(&a), before);
    assert!(a
        .patterns
        .workspace
        .error
        .as_ref()
        .unwrap()
        .contains("in use"));
}
