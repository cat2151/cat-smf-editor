use super::*;
use crate::view::palette::tests::{frame, press};
use eframe::egui::{self, Key};

#[test]
fn pattern_mode_v_from_root_timeline_and_song_uses_saved_music() {
    for surface in 0..3 {
        let (mut a, _, id) = edit_tests::fixture();
        library::edit(&mut a, id).unwrap();
        let c = egui::Context::default();
        if surface == 1 {
            press(&mut a, &c, Key::R);
            assert!(a.piano_roll.open);
        } else if surface == 2 {
            matrix::open(&mut a).unwrap();
        }
        let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
        press(&mut a, &c, Key::V);
        assert!(!a.note_mode, "surface {surface}");
        assert!(a.patterns.editing.is_none());
        assert!(a.loaded.as_ref().unwrap().display_music.is_none());
        assert!(presentation::name(&a, a.track).is_none());
        assert_eq!(
            cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
            before
        );
        assert!(!a.undo.available());
        assert!(!a.mml_input.open && a.note_edit.is_none());
    }
}

#[test]
fn track_changes_revoke_projection_and_source_authority_before_next_frame() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    assert!(a.can_edit_notes(source));
    let generation = a.generation;
    a.select_track(0);
    assert_eq!(a.generation, generation);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
    a.select_track(source);
    assert!(!a.can_edit_notes(source));
    assert!(presentation::name(&a, source).is_none());
    assert!(a.edit_music(|_| Ok(())).is_err());
    let c = egui::Context::default();
    track_selected(&mut a);
    frame(&mut a, &c, vec![]);
    library::edit(&mut a, id).unwrap();
    assert!(a.can_edit_notes(source));
    assert!(presentation::name(&a, source).is_some());
    assert!(a.loaded.as_ref().unwrap().display_music.is_some());
}

#[test]
fn stale_context_alone_never_grants_source_permission_or_pattern_identity() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    a.patterns.edit_song = a.song_id.wrapping_add(1);
    assert!(!a.can_edit_notes(source));
    assert!(presentation::name(&a, source).is_none());
    refresh(&mut a);
    assert!(a.patterns.editing.is_none());
    assert!(a.loaded.as_ref().unwrap().display_music.is_none());
}

#[test]
fn delayed_source_note_edit_cannot_commit_after_switching_to_song_track() {
    let (mut a, source, id) = edit_tests::fixture();
    library::edit(&mut a, id).unwrap();
    let index = a.loaded.as_ref().unwrap().visible().tracks[source]
        .events
        .iter()
        .position(cat_smf_editor::score::note_view::note_on)
        .unwrap();
    crate::view::note_edit::begin(
        &mut a,
        crate::view::note_edit::Field::Velocity,
        source,
        index,
        egui::Rect::from_min_size(egui::pos2(400.0, 200.0), egui::vec2(80.0, 25.0)),
        "51",
    );
    assert!(a.note_edit.is_some());
    let before = cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap();
    a.select_track(0);
    let c = egui::Context::default();
    press(&mut a, &c, Key::Enter);
    frame(&mut a, &c, vec![]);
    assert_eq!(
        cat_smf_editor::smf_io::encode(&a.loaded.as_ref().unwrap().music).unwrap(),
        before
    );
    assert!(!a.undo.available());
    assert!(a.note_edit.is_none());
}

#[test]
fn opening_a_song_with_patterns_shows_its_song_window() {
    let root = std::env::temp_dir().join(format!("smf-open-song-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let (patterned, _, _) = edit_tests::fixture();
    let patterned = patterned.loaded.unwrap().music;
    let mut plain = patterned.clone();
    plain.tracks.truncate(1);
    for (name, music, song) in [("patterned", &patterned, true), ("plain", &plain, false)] {
        let path = root.join(format!("{name}.mid"));
        std::fs::write(&path, cat_smf_editor::smf_io::encode(music).unwrap()).unwrap();
        let mut a = crate::view::tests::app(plain.clone());
        a.loaded = None;
        a.note_mode = false;
        a.saved = crate::store::saved::Document::new(Some(root.join("config")));
        a.config = crate::store::config::Store::open(root.join("status.json"));
        a.open_mid(path);
        assert!(a.load_error.is_none(), "{name}: {:?}", a.load_error);
        assert_eq!(mode::workspace_open(&a), song, "{name}");
        assert_eq!(a.patterns.workspace.song_cell.focus, song, "{name}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
