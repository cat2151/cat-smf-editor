use super::*;
use crate::view::palette::tests::{app, frame, press};
use cat_smf_editor::score::pattern::{operations, Catalog};
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn containers() -> App {
    let mut a = app();
    a.edit_tracks(|m| {
        operations::create_source(m)?;
        operations::create_source(m)?;
        Ok(())
    })
    .unwrap();
    a
}
#[test]
fn container_choice_enter_and_escape_are_keyboard_owned_and_cancel_is_nonmutating() {
    let mut a = containers();
    let c = egui::Context::default();
    let before = bytes(&a);
    open(&mut a).unwrap();
    frame(&mut a, &c, vec![]);
    assert!(a.patterns.confirming_creation());
    assert_eq!(bytes(&a), before);
    for k in [Key::I, Key::P, Key::A] {
        press(&mut a, &c, k);
        assert!(!a.mml_input.open);
        assert_eq!(bytes(&a), before);
    }
    press(&mut a, &c, Key::ArrowDown);
    let Choice::Container(chosen) = a.patterns.first.as_ref().unwrap().choices[1].0 else {
        panic!("second row is an existing container");
    };
    press(&mut a, &c, Key::Enter);
    assert!(!a.patterns.confirming_creation());
    let id = a.patterns.selected.unwrap();
    assert_eq!(
        Catalog::read(&a.loaded.as_ref().unwrap().music)
            .unwrap()
            .patterns[&id]
            .container,
        chosen
    );
    assert!(a.patterns.editing.as_ref().unwrap().standalone);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
    open(&mut a).unwrap();
    press(&mut a, &c, Key::Escape);
    assert!(!a.patterns.confirming_creation());
    assert_eq!(bytes(&a), before);
}
#[test]
fn first_choice_and_cancel_leave_normal_song_and_undo_intact() {
    let mut a = app();
    let c = egui::Context::default();
    let before = bytes(&a);
    let undo = a.undo.available();
    open(&mut a).unwrap();
    frame(&mut a, &c, vec![]);
    assert_eq!(a.patterns.first.as_ref().unwrap().choices.len(), 2);
    assert_eq!(bytes(&a), before);
    press(&mut a, &c, Key::Escape);
    assert_eq!(bytes(&a), before);
    assert_eq!(a.undo.available(), undo);
}
#[test]
fn initial_pair_uses_supplied_drum_settings_and_preserves_normal_tracks_atomically() {
    let mut a = app();
    let normal = bytes(&a);
    let count = a.loaded.as_ref().unwrap().music.tracks.len();
    let patch = cat_smf_editor::score::instrument::Patch {
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
    };
    library::initialize_with(&mut a, true, patch).unwrap();
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = Catalog::read(music).unwrap();
    assert_eq!(catalog.sources.len(), 2);
    assert_eq!(music.tracks.len(), count + 2);
    let p = &catalog.patterns[&a.patterns.selected.unwrap()];
    assert_eq!(music.tracks[p.track].name().as_deref(), Some("Drums"));
    assert!(a.loaded.as_ref().unwrap().patches[p.track].drums.is_some());
    let mut original = music.clone();
    original.tracks.truncate(count);
    assert_eq!(cat_smf_editor::smf_io::encode(&original).unwrap(), normal);
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), normal);
}
fn shows_heading(a: &mut App) -> bool {
    let c = egui::Context::default();
    let mut shown = false;
    for _ in 0..2 {
        let out = c.run(Default::default(), |ctx| show(a, ctx));
        shown = out.shapes.iter().any(|s| {
            matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "楽器を選んでください")
        });
    }
    shown
}
#[test]
fn instrument_choice_has_title_and_container_choice_does_not() {
    let mut a = app();
    open(&mut a).unwrap();
    assert!(shows_heading(&mut a));
    let mut a = containers();
    open(&mut a).unwrap();
    assert!(!shows_heading(&mut a));
}
#[test]
fn first_drum_pattern_keeps_song_visible_while_the_grid_owns_keys() {
    let mut a = app();
    let c = egui::Context::default();
    let patch = cat_smf_editor::score::instrument::Patch {
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
    };
    library::initialize_with(&mut a, true, patch).unwrap();
    let _ = c.run(Default::default(), |ctx| library::input(&mut a, ctx));
    frame(&mut a, &c, vec![]);
    assert!(a.drum_editor.open);
    assert_eq!(crate::view::commands::owner(&a, &c), Screen::Drums);
    assert!(crate::view::analysis::companion_visible(&a, &c));
    assert!(!a.patterns.workspace.song_cell.focus);
    let id = a.patterns.editing.as_ref().unwrap().pattern;
    assert!(
        super::super::workspace::choices(&a, super::super::workspace::Pane::Song)
            .iter()
            .any(|(p, _)| *p
                == cat_smf_editor::score::scene::arrangement::PatternOrScene::Pattern(id))
    );
    let track = a.track;
    press(&mut a, &c, Key::Enter);
    assert!(cat_smf_editor::score::drums::active(
        &a.loaded.as_ref().unwrap().visible().tracks[track],
        480,
        0,
        0
    ));
}
#[test]
fn new_rows_appear_only_when_no_container_of_that_kind_is_unused() {
    let mut a = containers();
    let rows: Vec<_> = choices(&a).unwrap().into_iter().map(|(c, _)| c).collect();
    assert!(!rows.contains(&Choice::New(false)));
    assert!(rows.contains(&Choice::New(true)));
    let patch = cat_smf_editor::score::instrument::Patch {
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
    };
    library::add_container_with(&mut a, Some(patch)).unwrap();
    assert!(!choices(&a)
        .unwrap()
        .iter()
        .any(|(c, _)| *c == Choice::New(true)));
}
#[test]
fn new_container_gets_a_numbered_name_and_undoes_with_its_pattern() {
    let mut a = containers();
    let before = bytes(&a);
    library::add_container_with(&mut a, None).unwrap();
    let music = &a.loaded.as_ref().unwrap().music;
    let p = &Catalog::read(music).unwrap().patterns[&a.patterns.selected.unwrap()];
    assert_eq!(music.tracks[p.track].name().as_deref(), Some("Init Saw 2"));
    a.undo_music().unwrap();
    assert_eq!(bytes(&a), before);
}
