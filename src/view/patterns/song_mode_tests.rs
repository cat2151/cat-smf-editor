use super::*;
use crate::view::palette::tests::{frame, press};
use eframe::egui::{self, Key};

fn framed(a: &App, track: usize) -> bool {
    presentation::frame(a, track).stroke.width > 0.0
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}
fn bars(count: usize) -> Vec<cat_smf_editor::score::scene::Meter> {
    let bar = cat_smf_editor::score::scene::Meter {
        numerator: 4,
        exponent: 2,
        ticks: BAR,
    };
    vec![bar; count]
}
fn last_pattern(a: &App) -> u64 {
    let music = &a.loaded.as_ref().unwrap().music;
    let catalog = cat_smf_editor::score::pattern::Catalog::read(music).unwrap();
    catalog.patterns.values().max_by_key(|p| p.tick).unwrap().id
}
const BAR: u64 = 1920;

#[test]
fn source_frame_precedes_edit_target() {
    let (mut a, source, id) = edit_tests::fixture();
    let c = egui::Context::default();
    let before = bytes(&a);
    assert!(!framed(&a, 0));
    assert!(!framed(&a, source));
    a.select_track(source);
    assert!(mode::workspace_open(&a));
    track_selected(&mut a);
    assert!(mode::workspace_open(&a));
    assert!(framed(&a, source));
    assert_eq!(mode::editing(&a, source).unwrap().pattern, id);
    assert!(framed(&a, source) && a.can_edit_notes(source));
    press(&mut a, &c, Key::I);
    frame(&mut a, &c, vec![]);
    assert!(a.mml_input.open);
    assert!(!crate::view::analysis::companion_visible(&a, &c));
    assert!(mode::workspace_open(&a) && framed(&a, source));
    press(&mut a, &c, Key::Escape);
    assert!(mode::workspace_open(&a) && framed(&a, source));
    assert_eq!(bytes(&a), before);
    a.select_track(0);
    a.select_track(source);
    track_selected(&mut a);
    assert!(framed(&a, source));
    assert!(a.can_edit_notes(source));
}

#[test]
fn losing_a_target_never_changes_song_lifetime_or_its_source_frame() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    a.patterns.edit_song = a.song_id.wrapping_add(1);
    assert!(framed(&a, source), "a stale target is not a window close");
    assert!(!a.can_edit_notes(source));
    refresh(&mut a);
    assert!(a.patterns.editing.is_none());
    assert!(framed(&a, source));
    a.select_track(0);
    assert!(!framed(&a, 0) && !framed(&a, source));
    assert!(mode::workspace_open(&a));
}

#[test]
fn raw_ends_the_projection_and_actual_source_edits_do_not_depend_on_a_frame() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    let c = egui::Context::default();
    let before = bytes(&a);
    press(&mut a, &c, Key::V);
    assert!(!a.note_mode && !mode::workspace_open(&a));
    assert!(!framed(&a, source));
    assert!(a.can_edit_notes(source));
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    press(&mut a, &c, Key::V);
    assert!(a.note_mode && mode::workspace_open(&a));
    assert!(framed(&a, source) && a.can_edit_notes(source));
    assert_eq!(mode::editing(&a, source).unwrap().pattern, id);
    assert_eq!(bytes(&a), before);
    assert!(!a.undo.available());
}

#[test]
fn shrinking_the_edited_pattern_brings_a_cursor_past_its_new_end_back() {
    use cat_smf_editor::score::pattern::operations::resize;
    let (mut a, _, _) = edit_tests::fixture();
    let id = last_pattern(&a);
    workspace::mutate(&mut a, |m| resize(m, id, bars(2))).unwrap();
    library::edit(&mut a, id).unwrap();
    crate::cursor::moves::at_tick(&mut a, BAR + 480);
    validate(&mut a);
    assert!(a.patterns.editing.is_some());
    workspace::mutate(&mut a, |m| resize(m, id, bars(1))).unwrap();
    crate::cursor::moves::at_tick(&mut a, BAR + 480);
    validate(&mut a);
    assert_eq!(a.patterns.editing.as_ref().unwrap().pattern, id);
    assert!(crate::play::playback::location(&a).0 < BAR);
}

#[test]
fn a_selected_track_other_than_the_edited_one_releases_the_edit() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    validate(&mut a);
    assert_eq!(a.patterns.editing.as_ref().map(|c| c.track), Some(source));
    a.track = 0;
    validate(&mut a);
    assert!(a.patterns.editing.is_none());
}

#[test]
fn the_edited_pattern_is_headed_by_its_name_and_its_containers_layers() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    let loaded = a.loaded.as_ref().unwrap();
    let catalog = cat_smf_editor::score::pattern::Catalog::read(&loaded.music).unwrap();
    assert_eq!(
        presentation::name(&a, source).as_ref(),
        Some(&catalog.patterns[&id].name)
    );
    let layers = loaded.patches[source]
        .layers
        .iter()
        .map(|l| l.label())
        .collect::<Vec<_>>()
        .join(" · ");
    assert!(!layers.is_empty());
    let output = egui::Context::default().run(Default::default(), |c| {
        egui::CentralPanel::default().show(c, |ui| heading(&a, ui, source));
    });
    assert!(output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == layers)));
}
