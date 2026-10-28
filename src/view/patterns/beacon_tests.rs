//! The first pattern's steps glow in turn: edit it, enter a beat, place it.
use super::workspace::Pane;
use super::*;
use eframe::egui::{self, Key, Modifiers};

fn frame(a: &mut App, c: &egui::Context, id: egui::ViewportId, events: Vec<egui::Event>) {
    let root = if id == egui::ViewportId::ROOT {
        events
    } else {
        c.data_mut(|d| d.insert_temp(egui::Id::new(("workspace-input", id)), events));
        vec![]
    };
    let mut input = egui::RawInput {
        events: root,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1500.0, 900.0),
        )),
        ..Default::default()
    };
    input.viewports.insert(
        egui::ViewportId::ROOT,
        egui::ViewportInfo {
            focused: Some(true),
            ..Default::default()
        },
    );
    let _ = c.run(input, |ctx| crate::view::show(a, ctx));
}
fn key(a: &mut App, c: &egui::Context, id: egui::ViewportId, k: Key, m: Modifiers) {
    let events = [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key: k,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: m,
        })
        .collect();
    frame(a, c, id, events);
}
/// The rectangle a beacon recorded in a fresh frame of `id`.
fn shown<T: Clone + Send + Sync + 'static>(
    a: &mut App,
    c: &egui::Context,
    id: egui::ViewportId,
    name: egui::Id,
) -> Option<T> {
    c.data_mut(|d| d.remove::<T>(name));
    frame(a, c, id, vec![]);
    c.data(|d| d.get_temp::<T>(name))
}
fn edit_button(a: &mut App, c: &egui::Context) -> Option<egui::Rect> {
    let song = Pane::Song.viewport();
    shown(
        a,
        c,
        song,
        egui::Id::new((Pane::Song.title(), "beacon-edit")),
    )
}
fn drum_beat(a: &mut App, c: &egui::Context) -> Option<egui::Rect> {
    shown(a, c, egui::ViewportId::ROOT, egui::Id::new("drum-beacon"))
}
fn place(a: &mut App, c: &egui::Context) -> Option<(egui::Rect, egui::Rect)> {
    let song = Pane::Song.viewport();
    shown(a, c, song, egui::Id::new("song-beacon-place"))
}
fn click(a: &mut App, c: &egui::Context, id: egui::ViewportId, pos: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            a,
            c,
            id,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
fn drum_patch() -> cat_smf_editor::score::instrument::Patch {
    cat_smf_editor::score::instrument::Patch {
        drums: Some(Default::default()),
        layers: (0..4)
            .map(|i| cat_smf_editor::score::instrument::ResolvedLayer {
                settings: cat_smf_editor::instrument_layers::Layer {
                    id: i + 1,
                    ..Default::default()
                },
                instrument: None,
            })
            .collect(),
        ..Default::default()
    }
}
/// The state right after New MID → new pattern → Drums.
fn first_drum() -> (App, egui::Context) {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let c = super::workspace_tests::context();
    library::initialize_with(&mut a, true, drum_patch()).unwrap();
    let _ = c.run(Default::default(), |ctx| library::input(&mut a, ctx));
    frame(&mut a, &c, egui::ViewportId::ROOT, vec![]);
    assert!(a.drum_editor.open);
    (a, c)
}

#[test]
fn first_drum_pattern_glows_edit_then_beat_then_placement() {
    let (mut a, c) = first_drum();
    let root = egui::ViewportId::ROOT;
    let song = Pane::Song.viewport();
    assert!(drum_beat(&mut a, &c).is_some());
    assert!(place(&mut a, &c).is_none());
    // Leaving the grid for Song shows Edit on the empty candidate.
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert!(!a.drum_editor.open);
    let button = edit_button(&mut a, &c).expect("Edit on the empty pattern");
    click(&mut a, &c, song, button.center());
    frame(&mut a, &c, root, vec![]);
    assert!(a.drum_editor.open, "{:?}", a.patterns.workspace.error);
    assert!(drum_beat(&mut a, &c).is_some());
    // One step makes the pattern a part to place.
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    assert!(drum_beat(&mut a, &c).is_none());
    assert!(edit_button(&mut a, &c).is_none());
    let (card, cell) = place(&mut a, &c).expect("drag from the pattern to lane 1 meas. 1");
    let first = c
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new((Pane::Song.title(), "cell", 0usize, 1u64)))
        })
        .unwrap();
    assert_eq!(cell, first);
    assert!(card.top() > cell.bottom());
    // Enter puts the candidate at the selected first cell, and the glow ends.
    key(&mut a, &c, song, Key::Enter, Modifiers::NONE);
    let collection = cat_smf_editor::score::scene::read(&a.loaded.as_ref().unwrap().music).unwrap();
    assert_eq!(collection.song.len(), 1);
    assert_eq!(collection.song[0].measure, 1);
    assert!(place(&mut a, &c).is_none());
}

#[test]
fn shift_e_opens_the_drum_editor_for_a_drum_pattern_with_notes() {
    let (mut a, c) = first_drum();
    let root = egui::ViewportId::ROOT;
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    key(&mut a, &c, root, Key::Escape, Modifiers::NONE);
    assert!(!a.drum_editor.open);
    key(&mut a, &c, Pane::Song.viewport(), Key::E, Modifiers::SHIFT);
    frame(&mut a, &c, root, vec![]);
    assert!(a.drum_editor.open);
}

#[test]
fn edit_on_an_empty_saw_pattern_opens_mml_input_and_notes_keep_the_event_list() {
    let mut a = crate::view::tests::app(cat_smf_editor::edit::tracks::new_song());
    let c = super::workspace_tests::context();
    library::initialize_with(&mut a, false, drum_patch()).unwrap();
    let root = egui::ViewportId::ROOT;
    let song = Pane::Song.viewport();
    frame(&mut a, &c, root, vec![]);
    assert!(!a.mml_input.open);
    let button = edit_button(&mut a, &c).expect("Edit on the empty pattern");
    click(&mut a, &c, song, button.center());
    for _ in 0..3 {
        frame(&mut a, &c, root, vec![]);
    }
    assert!(a.mml_input.open, "{:?}", a.load_error);
    for t in ["c", "d", "e"] {
        frame(&mut a, &c, root, vec![egui::Event::Text(t.into())]);
    }
    key(&mut a, &c, root, Key::Enter, Modifiers::NONE);
    for _ in 0..3 {
        frame(&mut a, &c, root, vec![]);
    }
    assert!(!a.mml_input.open, "{:?}", a.load_error);
    assert!(edit_button(&mut a, &c).is_none());
    key(&mut a, &c, song, Key::E, Modifiers::SHIFT);
    for _ in 0..3 {
        frame(&mut a, &c, root, vec![]);
    }
    assert!(!a.mml_input.open);
    assert!(!a.drum_editor.open);
}
