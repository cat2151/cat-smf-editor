use super::*;
use crate::view::palette::tests::press;
use eframe::egui::{self, Key};

fn twice(entered: bool, expire: bool) {
    let (mut a, source, id) = edit_tests::fixture();
    a.select_track(source);
    track_selected(&mut a);
    let c = egui::Context::default();
    if entered {
        library::edit(&mut a, id).unwrap();
        assert_eq!(mode::editing(&a, source).unwrap().pattern, id);
    } else {
        assert!(a.patterns.editing.is_none());
    }
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    press(&mut a, &c, Key::V);
    assert!(!a.note_mode && !mode::workspace_open(&a));
    if expire {
        press(&mut a, &c, Key::End);
    }
    press(&mut a, &c, Key::V);
    assert!(a.note_mode);
    assert_eq!(a.track, source);
    assert!(
        mode::workspace_open(&a),
        "Notes + actual source must always activate pattern mode"
    );
    assert!(mode::source(&a, source));
    assert!(presentation::frame(&a, source).stroke.width > 0.0);
    assert!(mode::workspace_open(&a));
    assert_eq!(
        a.can_edit_notes(source),
        mode::editing(&a, source).is_some()
    );
    if !entered || expire {
        assert!(a.patterns.editing.is_none());
        assert!(!a.can_edit_notes(source));
    }
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!a.undo.available());
}

#[test]
fn v_twice_before_enter_always_returns_source_to_pattern_mode() {
    twice(false, false);
}
#[test]
fn v_twice_after_independent_edit_returns_source_to_pattern_mode() {
    twice(true, false);
}
#[test]
fn v_twice_with_expired_edit_target_still_returns_source_to_pattern_mode() {
    twice(true, true);
}

#[test]
fn selecting_and_restoring_a_notes_source_never_need_an_edit_context_or_open_flag() {
    let (mut a, source, _) = edit_tests::fixture();
    a.select_track(source);
    assert!(mode::source(&a, source));
    assert!(!a.can_edit_notes(source));
    let session = a.session().unwrap();
    let music = a.loaded.as_ref().unwrap().music.clone();
    let mut restarted = crate::view::tests::app(music);
    restarted.restore_position(&session);
    assert_eq!(restarted.track, source);
    assert!(restarted.note_mode && mode::source(&restarted, source));
    assert!(restarted.patterns.editing.is_none());
    assert!(!restarted.can_edit_notes(source));
    restarted.select_track(0);
    assert!(!mode::source(&restarted, 0));
    restarted.select_track(source);
    assert!(mode::source(&restarted, source));
}

#[test]
fn undo_restoring_a_different_real_source_keeps_notes_in_pattern_mode() {
    let (mut a, source, _) = edit_tests::fixture();
    let mut added = None;
    a.edit_tracks(|m| {
        added = Some(cat_smf_editor::score::pattern::operations::create_source(
            m,
        )?);
        Ok(())
    })
    .unwrap();
    a.select_track(added.unwrap());
    assert!(a.note_mode && mode::source(&a, a.track));
    a.undo_music().unwrap();
    assert_eq!(a.track, source);
    assert!(a.note_mode && mode::source(&a, source));
    assert!(a.patterns.editing.is_none());
    assert!(!a.can_edit_notes(source));
}
