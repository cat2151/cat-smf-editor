use super::*;
use crate::view::palette::tests::{frame, press};
use eframe::egui::{self, Key};

fn framed(a: &App, track: usize) -> bool {
    presentation::frame(a, track).stroke.width > 0.0
}
fn bytes(a: &App) -> Vec<u8> {
    cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap()
}

#[test]
fn source_frame_precedes_edit_target_and_close_revokes_both_projection_and_authority() {
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
    assert!(a.patterns.editing.is_none());
    assert!(presentation::name(&a, source).is_none());
    assert!(!a.can_edit_notes(source));
    assert!(a.edit_music(|_| Ok(())).is_err());
    library::edit(&mut a, id).unwrap();
    assert_eq!(mode::editing(&a, source).unwrap().pattern, id);
    assert!(framed(&a, source) && a.can_edit_notes(source));
    press(&mut a, &c, Key::I);
    frame(&mut a, &c, vec![]);
    assert!(a.mml_input.open);
    assert!(!crate::view::analysis::companion_visible(&a, &c));
    assert!(mode::workspace_open(&a) && framed(&a, source));
    press(&mut a, &c, Key::Escape);
    assert!(mode::workspace_open(&a) && framed(&a, source));
    mode::close_workspace(&mut a);
    assert!(!mode::workspace_open(&a) && !framed(&a, source));
    assert!(a.note_mode, "closing Song must preserve Notes");
    assert_eq!(a.track, 0);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    assert!(!a.can_edit_notes(source));
    assert_eq!(bytes(&a), before);
    a.select_track(source);
    track_selected(&mut a);
    assert!(framed(&a, source));
    assert!(!a.can_edit_notes(source));
}

#[test]
fn losing_a_target_never_changes_song_lifetime_or_its_source_frame() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    crate::cursor::moves::at_tick(&mut a, 3840);
    validate(&mut a);
    assert!(a.patterns.editing.is_none());
    assert!(mode::workspace_open(&a) && framed(&a, source));
    assert!(!a.can_edit_notes(source));
    assert!(mode::workspace_open(&a));
    library::edit(&mut a, id).unwrap();
    a.patterns.edit_song = a.song_id.wrapping_add(1);
    assert!(framed(&a, source), "a stale target is not a window close");
    assert!(!a.can_edit_notes(source));
    refresh(&mut a);
    assert!(a.patterns.editing.is_none());
    assert!(framed(&a, source));
    a.select_track(0);
    assert!(!framed(&a, 0) && !framed(&a, source));
    assert!(!mode::workspace_open(&a));
}

#[test]
fn raw_closes_song_and_actual_source_edits_do_not_depend_on_a_frame() {
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
