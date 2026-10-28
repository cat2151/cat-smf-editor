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
    assert!(!a.patterns.workspace_open);
    assert!(!a.can_edit_notes(source));
    let session = a.session().unwrap();
    let music = a.loaded.as_ref().unwrap().music.clone();
    let mut restarted = crate::view::tests::app(music);
    restarted.restore_position(&session);
    assert_eq!(restarted.track, source);
    assert!(restarted.note_mode && mode::source(&restarted, source));
    assert!(!restarted.patterns.workspace_open);
    assert!(restarted.patterns.editing.is_none());
    assert!(!restarted.can_edit_notes(source));
    restarted.select_track(0);
    assert!(!mode::workspace_open(&restarted));
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

#[test]
fn closing_song_on_a_song_destination_keeps_notes_and_saved_music() {
    let (mut a, _, _) = edit_tests::fixture();
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    mode::close_workspace(&mut a);
    assert_eq!(a.track, 0);
    assert!(a.note_mode && !mode::workspace_open(&a));
    assert!(a.patterns.editing.is_none());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!a.undo.available());
}

#[test]
fn closing_source_song_skips_other_sources_without_changing_notes_music_or_undo() {
    let (a, source, _) = edit_tests::fixture();
    let mut music = a.loaded.as_ref().unwrap().music.clone();
    let second = cat_smf_editor::score::pattern::operations::create_source(&mut music).unwrap();
    let mut a = crate::view::tests::app(music);
    a.select_track(second);
    crate::cursor::moves::at_tick(&mut a, 120);
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    let generation = a.generation;
    assert!(mode::actual_source(&a, source) && mode::source(&a, second));
    mode::close_workspace(&mut a);
    assert_eq!(a.track, 0, "skip every source, not just the current track");
    assert!(a.note_mode && !mode::workspace_open(&a));
    assert_eq!(crate::play::playback::location(&a).0, 120);
    assert!(a.patterns.editing.is_none() && a.patterns.raw_context.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    assert_eq!(a.generation, generation);
    assert!(!a.undo.available());
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
}
